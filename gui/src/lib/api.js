import { globalState } from './state.svelte.js';

const isTauri = typeof window !== 'undefined' && (!!window.__TAURI_INTERNALS__ || location.protocol === 'tauri:' || location.protocol === 'file:');
const API_BASE = isTauri ? 'http://127.0.0.1:8002' : '';

class ApiClient {
    async request(endpoint, options = {}) {
        const url = API_BASE + endpoint;
        const headers = new Headers(options.headers || {});
        
        if (globalState.authToken) {
            headers.set('Authorization', `Bearer ${globalState.authToken}`);
        }

        // La clave de sync viaja solo en las llamadas de sincronización para que
        // el servidor descifre la config remota en tránsito.
        if (globalState.syncKeyHex && endpoint.startsWith('/sync/')) {
            headers.set('X-Pact-Sync-Key', globalState.syncKeyHex);
        }
        
        if (options.body && !(options.body instanceof FormData)) {
            headers.set('Content-Type', 'application/json');
        }

        const config = {
            ...options,
            headers
        };

        try {
            console.log(`[API] ${options.method || 'GET'} ${url}`, options.body || '');
            const response = await fetch(url, config);
            console.log(`[API] Response ${url}: ${response.status}`, response.statusText);
            
            if (response.status === 401) {
                if (globalState.authToken) {
                    globalState.forceLock("Sesión expirada o no autorizada.");
                }
                throw new Error("Unauthorized");
            }

            if (!response.ok) {
                const errorData = await response.json().catch(() => ({}));
                const message = errorData.error || `Error ${response.status}: ${response.statusText}`;
                
                // Notificar automáticamente errores si no es una petición silenciosa
                if (!options.silent) {
                    globalState.notify("Error de Servidor", message, "error");
                }
                
                throw new Error(message);
            }

            // Si es 204 No Content, no intentamos parsear JSON
            if (response.status === 204) return null;
            
            const text = await response.text();
            if (!text) return null;

            try {
                return JSON.parse(text);
            } catch (e) {
                return text;
            }
        } catch (error) {
            if (error.message !== "Unauthorized") {
                console.error(`[API] Error in ${url}:`, error);
            }
            throw error;
        }
    }

    get(endpoint, options = {}) {
        return this.request(endpoint, { ...options, method: 'GET' });
    }

    post(endpoint, body, options = {}) {
        return this.request(endpoint, { 
            ...options, 
            method: 'POST', 
            body: body instanceof FormData ? body : JSON.stringify(body) 
        });
    }

    put(endpoint, body, options = {}) {
        return this.request(endpoint, { 
            ...options, 
            method: 'PUT', 
            body: body instanceof FormData ? body : JSON.stringify(body) 
        });
    }

    patch(endpoint, body, options = {}) {
        return this.request(endpoint, {
            ...options,
            method: 'PATCH',
            body: body instanceof FormData ? body : JSON.stringify(body)
        });
    }

    delete(endpoint, body, options = {}) {
        return this.request(endpoint, { 
            ...options, 
            method: 'DELETE',
            body: body instanceof FormData ? body : (body ? JSON.stringify(body) : undefined)
        });
    }
}

export const api = new ApiClient();
