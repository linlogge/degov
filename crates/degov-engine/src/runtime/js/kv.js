// KV Database JavaScript API
const { op_kv_get, op_kv_set } = Deno.core.ops;

class KvDatabase {
    constructor() {}

    async get(key) {
        const result = await op_kv_get(key);
        if (result && result.length > 0) {
            const text = new TextDecoder().decode(new Uint8Array(result));
            try {
                return JSON.parse(text);
            } catch (e) {
                return text;
            }
        }
        return null;
    }

    async set(key, value) {
        const jsonValue = typeof value === 'string' ? value : JSON.stringify(value);
        await op_kv_set(key, jsonValue);
    }

    async getJson(key) {
        return this.get(key);
    }

    async setJson(key, value) {
        return this.set(key, value);
    }
}

// Export KV API
globalThis.KV = {
    openKv: function() {
        return new KvDatabase();
    }
};

// Create a default database instance
globalThis.db = KV.openKv();
