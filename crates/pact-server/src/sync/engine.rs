use std::collections::HashMap;
use crate::db::Database;
use crate::schemas::SyncPlan;
use sqlx::Error;

pub struct SyncEngine<'a> {
    local_db: &'a dyn Database,
    remote_db: &'a dyn Database,
    user_id: i32,
}

impl<'a> SyncEngine<'a> {
    pub fn new(local_db: &'a dyn Database, remote_db: &'a dyn Database, user_id: i32) -> Self {
        Self {
            local_db,
            remote_db,
            user_id,
        }
    }

    pub async fn generate_plan(&self) -> Result<SyncPlan, Error> {
        let local_folders = self.local_db.get_folders_by_user(self.user_id).await?;
        let remote_folders = self.remote_db.get_folders_by_user(self.user_id).await?;

        let mut plan = SyncPlan {
            to_push_folders: Vec::new(),
            to_pull_folders: Vec::new(),
            conflict_folders: Vec::new(),
            to_push_accounts: Vec::new(),
            to_pull_accounts: Vec::new(),
            conflict_accounts: Vec::new(),
        };

        let mut local_map = HashMap::new();
        for f in &local_folders {
            local_map.insert(f.uid.clone(), f);
        }

        let mut remote_map = HashMap::new();
        for f in &remote_folders {
            remote_map.insert(f.uid.clone(), f);
        }

        // Reconcile folders
        for (uid, local_f) in &local_map {
            if let Some(remote_f) = remote_map.get(uid) {
                if local_f.version > remote_f.version {
                    plan.to_push_folders.push(uid.clone());
                } else if remote_f.version > local_f.version {
                    plan.to_pull_folders.push(uid.clone());
                }
            } else {
                plan.to_push_folders.push(uid.clone());
            }
        }

        for (uid, _) in &remote_map {
            if !local_map.contains_key(uid) {
                plan.to_pull_folders.push(uid.clone());
            }
        }

        // Reconcile accounts
        let local_accounts = self.local_db.get_accounts_by_user(self.user_id).await?;
        let remote_accounts = self.remote_db.get_accounts_by_user(self.user_id).await?;

        let mut local_acc_map = HashMap::new();
        for a in &local_accounts {
            local_acc_map.insert(a.uid.clone(), a);
        }

        let mut remote_acc_map = HashMap::new();
        for a in &remote_accounts {
            remote_acc_map.insert(a.uid.clone(), a);
        }

        for (uid, local_a) in &local_acc_map {
            if let Some(remote_a) = remote_acc_map.get(uid) {
                if local_a.version > remote_a.version {
                    plan.to_push_accounts.push(uid.clone());
                } else if remote_a.version > local_a.version {
                    plan.to_pull_accounts.push(uid.clone());
                }
            } else {
                plan.to_push_accounts.push(uid.clone());
            }
        }

        for (uid, _) in &remote_acc_map {
            if !local_acc_map.contains_key(uid) {
                plan.to_pull_accounts.push(uid.clone());
            }
        }

        Ok(plan)
    }

    pub async fn apply_plan(&self, plan: &SyncPlan) -> Result<(), Error> {
        // Important: folders must be synced before accounts to maintain foreign key integrity
        // In "Last Write Wins", we just update the destination with the source data.
        
        // 1. Pull folders (Remote -> Local)
        for uid in &plan.to_pull_folders {
            if let Some(remote_f) = self.remote_db.get_folder_by_uid(self.user_id, uid).await? {
                let create_f: crate::models::CreateFolder = remote_f.clone().into();
                if let Some(local_f) = self.local_db.get_folder_by_uid(self.user_id, uid).await? {
                    self.local_db.update_folder(local_f.id, &create_f, remote_f.version).await?;
                } else {
                    self.local_db.create_folder(&create_f).await?;
                }
            }
        }

        // 2. Push folders (Local -> Remote)
        for uid in &plan.to_push_folders {
            if let Some(local_f) = self.local_db.get_folder_by_uid(self.user_id, uid).await? {
                let create_f: crate::models::CreateFolder = local_f.clone().into();
                if let Some(remote_f) = self.remote_db.get_folder_by_uid(self.user_id, uid).await? {
                    self.remote_db.update_folder(remote_f.id, &create_f, local_f.version).await?;
                } else {
                    self.remote_db.create_folder(&create_f).await?;
                }
            }
        }

        // 3. Pull accounts (Remote -> Local)
        for uid in &plan.to_pull_accounts {
            if let Some(remote_a) = self.remote_db.get_account_by_uid(self.user_id, uid).await? {
                // Re-map folder_id to local context
                if let Some(remote_f) = self.remote_db.get_folders_by_user(self.user_id).await?.into_iter().find(|f| f.id == remote_a.folder_id) {
                    if let Some(local_f) = self.local_db.get_folder_by_uid(self.user_id, &remote_f.uid).await? {
                        let mut create_a: crate::models::CreateAccount = remote_a.clone().into();
                        create_a.folder_id = local_f.id;
                        
                        if let Some(local_a) = self.local_db.get_account_by_uid(self.user_id, uid).await? {
                            self.local_db.update_account(local_a.id, &create_a, remote_a.version).await?;
                        } else {
                            self.local_db.create_account(&create_a).await?;
                        }
                    }
                }
            }
        }

        // 4. Push accounts (Local -> Remote)
        for uid in &plan.to_push_accounts {
            if let Some(local_a) = self.local_db.get_account_by_uid(self.user_id, uid).await? {
                // Re-map folder_id to remote context
                if let Some(local_f) = self.local_db.get_folders_by_user(self.user_id).await?.into_iter().find(|f| f.id == local_a.folder_id) {
                    if let Some(remote_f) = self.remote_db.get_folder_by_uid(self.user_id, &local_f.uid).await? {
                        let mut create_a: crate::models::CreateAccount = local_a.clone().into();
                        create_a.folder_id = remote_f.id;
 
                        if let Some(remote_a) = self.remote_db.get_account_by_uid(self.user_id, uid).await? {
                            self.remote_db.update_account(remote_a.id, &create_a, local_a.version).await?;
                        } else {
                            self.remote_db.create_account(&create_a).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
