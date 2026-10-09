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

        for uid in remote_map.keys() {
            if !local_map.contains_key(uid) {
                plan.to_pull_folders.push(uid.clone());
            }
        }

        // Reconcile accounts (exclude soft-deleted)
        let local_accounts = self.local_db.get_accounts_by_user(self.user_id).await?;
        let local_accounts: Vec<_> = local_accounts.into_iter().filter(|a| a.deleted_at.is_none()).collect();
        let remote_accounts = self.remote_db.get_accounts_by_user(self.user_id).await?;
        let remote_accounts: Vec<_> = remote_accounts.into_iter().filter(|a| a.deleted_at.is_none()).collect();

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

        for uid in remote_acc_map.keys() {
            if !local_acc_map.contains_key(uid) {
                plan.to_pull_accounts.push(uid.clone());
            }
        }

        Ok(plan)
    }

    pub async fn apply_plan(&self, plan: &SyncPlan) -> Result<(), Error> {
        // Important: folders must be synced before accounts to maintain foreign key integrity
        // In "Last Write Wins", we just update the destination with the source data.
        
        // Batch-fetch all folders and accounts to avoid N+1 queries (exclude soft-deleted)
        let local_folders = self.local_db.get_folders_by_user(self.user_id).await?;
        let remote_folders = self.remote_db.get_folders_by_user(self.user_id).await?;
        let local_accounts = self.local_db.get_accounts_by_user(self.user_id).await?;
        let local_accounts: Vec<_> = local_accounts.into_iter().filter(|a| a.deleted_at.is_none()).collect();
        let remote_accounts = self.remote_db.get_accounts_by_user(self.user_id).await?;
        let remote_accounts: Vec<_> = remote_accounts.into_iter().filter(|a| a.deleted_at.is_none()).collect();

        let local_folders_by_uid: HashMap<String, _> = local_folders.iter().map(|f| (f.uid.clone(), f)).collect();
        let remote_folders_by_uid: HashMap<String, _> = remote_folders.iter().map(|f| (f.uid.clone(), f)).collect();
        let local_folders_by_id: HashMap<i32, _> = local_folders.iter().map(|f| (f.id, f)).collect(); 
        let remote_folders_by_id: HashMap<i32, _> = remote_folders.iter().map(|f| (f.id, f)).collect();
        let local_accounts_by_uid: HashMap<String, _> = local_accounts.iter().map(|a| (a.uid.clone(), a)).collect();
        let remote_accounts_by_uid: HashMap<String, _> = remote_accounts.iter().map(|a| (a.uid.clone(), a)).collect();

        // 1. Pull folders (Remote -> Local)
        for uid in &plan.to_pull_folders {
            if let Some(remote_f) = remote_folders_by_uid.get(uid) {
                let create_f: crate::models::CreateFolder = (*remote_f).clone().into();
                if let Some(local_f) = local_folders_by_uid.get(uid) {
                    self.local_db.update_folder(local_f.id, &create_f, remote_f.version).await?;
                } else {
                    self.local_db.create_folder(&create_f).await?;
                }
            }
        }

        // 2. Push folders (Local -> Remote)
        for uid in &plan.to_push_folders {
            if let Some(local_f) = local_folders_by_uid.get(uid) {
                let create_f: crate::models::CreateFolder = (*local_f).clone().into();
                if let Some(remote_f) = remote_folders_by_uid.get(uid) {
                    self.remote_db.update_folder(remote_f.id, &create_f, local_f.version).await?;
                } else {
                    self.remote_db.create_folder(&create_f).await?;
                }
            }
        }

        // 3. Pull accounts (Remote -> Local)
        for uid in &plan.to_pull_accounts {
            if let Some(remote_a) = remote_accounts_by_uid.get(uid) {
                let mut create_a: crate::models::CreateAccount = (*remote_a).clone().into();

                // Re-map folder_id to local context if it exists
                if let Some(fid) = remote_a.folder_id {
                    if let Some(remote_f) = remote_folders_by_id.get(&fid) {
                        if let Some(local_f) = local_folders_by_uid.get(&remote_f.uid) {
                            create_a.folder_id = Some(local_f.id);
                        }
                    }
                } else {
                    create_a.folder_id = None;
                }

                if let Some(local_a) = local_accounts_by_uid.get(uid) {
                    self.local_db.update_account(local_a.id, &create_a, remote_a.version).await?;
                } else {
                    self.local_db.create_account(&create_a).await?;
                }
            }
        }

        // 4. Push accounts (Local -> Remote)
        for uid in &plan.to_push_accounts {
            if let Some(local_a) = local_accounts_by_uid.get(uid) {
                let mut create_a: crate::models::CreateAccount = (*local_a).clone().into();

                // Re-map folder_id to remote context if it exists
                if let Some(fid) = local_a.folder_id {
                    if let Some(local_f) = local_folders_by_id.get(&fid) {
                        if let Some(remote_f) = remote_folders_by_uid.get(&local_f.uid) {
                            create_a.folder_id = Some(remote_f.id);
                        }
                    }
                } else {
                    create_a.folder_id = None;
                }

                if let Some(remote_a) = remote_accounts_by_uid.get(uid) {
                    self.remote_db.update_account(remote_a.id, &create_a, local_a.version).await?;
                } else {
                    self.remote_db.create_account(&create_a).await?;
                }
            }
        }

        Ok(())
    }
}
