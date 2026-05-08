/**
 * app.js — Sample JavaScript file for testing syntax highlighting.
 * Covers: classes, async/await, destructuring, template literals,
 * generators, Proxy, regex, JSDoc, and edge-case string content.
 */

'use strict';

// ── Constants and types ───────────────────────────────────────────────────────

const VERSION = '1.0.0';
const MAX_RETRIES = 3;
const COLORS = Object.freeze({
    RED:   '#ff6b6b',
    GREEN: '#51cf66',
    BLUE:  '#339af0',
});

/** @typedef {{ id: number, name: string, email: string }} User */

// ── Utility functions ─────────────────────────────────────────────────────────

/**
 * Clamps a value between min and max.
 * @param {number} val
 * @param {number} min
 * @param {number} max
 * @returns {number}
 */
const clamp = (val, min, max) => Math.min(Math.max(val, min), max);

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const escapeHtml = (str) =>
    str
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#039;');

// Regex with groups, flags, and lookahead
const EMAIL_RE = /^(?<user>[a-zA-Z0-9._%+\-]+)@(?<domain>[a-zA-Z0-9.\-]+)\.(?<tld>[a-zA-Z]{2,})$/i;
const URL_RE   = /https?:\/\/(?:www\.)?[-a-zA-Z0-9@:%._+~#=]{2,256}\.[a-z]{2,6}\b(?:[-a-zA-Z0-9@:%_+.~#?&/=]*)/g;

// ── EventEmitter (base class) ─────────────────────────────────────────────────

class EventEmitter {
    #listeners = new Map();

    on(event, fn) {
        const list = this.#listeners.get(event) ?? [];
        list.push(fn);
        this.#listeners.set(event, list);
        return this; // chainable
    }

    off(event, fn) {
        const list = this.#listeners.get(event) ?? [];
        this.#listeners.set(event, list.filter((f) => f !== fn));
        return this;
    }

    emit(event, ...args) {
        (this.#listeners.get(event) ?? []).forEach((fn) => fn(...args));
    }

    once(event, fn) {
        const wrapper = (...args) => { fn(...args); this.off(event, wrapper); };
        return this.on(event, wrapper);
    }
}

// ── ApiClient ─────────────────────────────────────────────────────────────────

class ApiClient extends EventEmitter {
    #baseUrl;
    #token;
    #cache = new Map();

    constructor(baseUrl, { token = null, timeout = 5000 } = {}) {
        super();
        this.#baseUrl = baseUrl.replace(/\/$/, '');
        this.#token   = token;
        this.timeout  = timeout;
    }

    get headers() {
        const h = { 'Content-Type': 'application/json' };
        if (this.#token) h['Authorization'] = `Bearer ${this.#token}`;
        return h;
    }

    async #request(method, path, body = null) {
        const url = `${this.#baseUrl}${path}`;
        const cacheKey = `${method}:${url}`;

        if (method === 'GET' && this.#cache.has(cacheKey)) {
            return this.#cache.get(cacheKey);
        }

        const controller = new AbortController();
        const timer = setTimeout(() => controller.abort(), this.timeout);

        try {
            const res = await fetch(url, {
                method,
                headers: this.headers,
                body: body ? JSON.stringify(body) : null,
                signal: controller.signal,
            });

            if (!res.ok) {
                const err = await res.json().catch(() => ({ message: res.statusText }));
                throw new ApiError(err.message ?? 'Unknown error', res.status);
            }

            const data = await res.json();
            if (method === 'GET') this.#cache.set(cacheKey, data);
            this.emit('response', { url, status: res.status, data });
            return data;
        } catch (e) {
            if (e.name === 'AbortError') throw new ApiError('Request timed out', 408);
            throw e;
        } finally {
            clearTimeout(timer);
        }
    }

    get(path)            { return this.#request('GET',    path); }
    post(path, body)     { return this.#request('POST',   path, body); }
    put(path, body)      { return this.#request('PUT',    path, body); }
    patch(path, body)    { return this.#request('PATCH',  path, body); }
    delete(path)         { return this.#request('DELETE', path); }

    invalidate(path) {
        this.#cache.delete(`GET:${this.#baseUrl}${path}`);
    }
}

// ── Custom error ──────────────────────────────────────────────────────────────

class ApiError extends Error {
    constructor(message, status) {
        super(message);
        this.name   = 'ApiError';
        this.status = status;
    }
}

// ── UserStore (with Proxy for reactivity) ────────────────────────────────────

function createReactive(target, onChange) {
    return new Proxy(target, {
        set(obj, prop, value) {
            const prev = obj[prop];
            obj[prop] = value;
            if (prev !== value) onChange(prop, value, prev);
            return true;
        },
        deleteProperty(obj, prop) {
            if (prop in obj) {
                delete obj[prop];
                onChange(prop, undefined, obj[prop]);
            }
            return true;
        },
    });
}

class UserStore extends EventEmitter {
    #api;
    #state;

    constructor(api) {
        super();
        this.#api = api;
        this.#state = createReactive({ users: [], loading: false, error: null }, (prop, val) => {
            this.emit('change', { prop, val });
        });
    }

    get users()   { return this.#state.users; }
    get loading() { return this.#state.loading; }
    get error()   { return this.#state.error; }

    async fetchAll() {
        this.#state.loading = true;
        this.#state.error   = null;
        try {
            const data = await this.#api.get('/users');
            this.#state.users = data;
            return data;
        } catch (e) {
            this.#state.error = e.message;
            throw e;
        } finally {
            this.#state.loading = false;
        }
    }

    async create(payload) {
        const user = await this.#api.post('/users', payload);
        this.#state.users = [...this.#state.users, user];
        this.#api.invalidate('/users');
        return user;
    }

    /** @param {number} id */
    findById(id) {
        return this.#state.users.find((u) => u.id === id) ?? null;
    }
}

// ── Generator: paginate ───────────────────────────────────────────────────────

async function* paginate(api, path, pageSize = 20) {
    let page = 1;
    while (true) {
        const { data, hasMore } = await api.get(`${path}?page=${page}&size=${pageSize}`);
        yield* data;
        if (!hasMore) break;
        page++;
    }
}

// ── Template literal tag ──────────────────────────────────────────────────────

function html(strings, ...values) {
    return strings.reduce((acc, str, i) => {
        const val = values[i - 1];
        return acc + (typeof val === 'string' ? escapeHtml(val) : (val ?? '')) + str;
    });
}

// ── Destructuring and spread edge cases ───────────────────────────────────────

const { RED: primaryColor, ...otherColors } = COLORS;

const matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
const [[a, , c], , [g, h, i]] = matrix;

function processUser({ id, name = 'Anonymous', roles: [firstRole = 'viewer', ...restRoles] = [] } = {}) {
    return { id, name, firstRole, restRoles };
}

// ── Symbols and WeakRef ───────────────────────────────────────────────────────

const _private = Symbol('private');
const _instance = Symbol.for('singleton.instance');

class Singleton {
    static [_instance] = null;

    static getInstance() {
        if (!Singleton[_instance]) {
            Singleton[_instance] = new Singleton();
        }
        return Singleton[_instance];
    }

    [_private]() { return 42; }
}

// ── String edge cases ─────────────────────────────────────────────────────────

const str1 = "Double \"quoted\" string with 'singles' inside";
const str2 = 'Single \'escaped\' with "doubles" inside';
const str3 = `Template with ${1 + 2} and nested \`backtick\` and ${`inner ${VERSION}`}`;
const str4 = String.raw`Raw: \n \t A — no escaping here`;
const str5 = "ABC 😀 \x41\x42"; // Unicode escapes

// ── Numeric literals ──────────────────────────────────────────────────────────

const dec  = 1_000_000;
const hex  = 0xFF_EC_D8_12;
const oct  = 0o777;
const bin  = 0b1010_0001;
const big  = 9007199254740991n;
const flt  = 1.234_567e-10;

// ── Async error handling with retry ──────────────────────────────────────────

async function withRetry(fn, retries = MAX_RETRIES, delay = 200) {
    for (let attempt = 1; attempt <= retries; attempt++) {
        try {
            return await fn();
        } catch (e) {
            if (attempt === retries || !(e instanceof ApiError) || e.status < 500) throw e;
            await sleep(delay * 2 ** (attempt - 1)); // exponential back-off
        }
    }
}

// ── Main entry point ──────────────────────────────────────────────────────────

async function main() {
    const api   = new ApiClient('https://api.example.com', { token: process.env.API_TOKEN });
    const store = new UserStore(api);

    store.on('change', ({ prop, val }) => console.log(`[store] ${prop} changed:`, val));
    api.on('response', ({ url, status }) => console.log(`[api] ${status} ${url}`));

    try {
        await withRetry(() => store.fetchAll());

        const { users } = store;
        const admins = users.filter(({ roles = [] }) => roles.includes('admin'));

        console.log(`Loaded ${users.length} users, ${admins.length} admins`);
        console.log(html`<p>Hello, ${'<script>alert(1)</script>'}</p>`);

        // Pagination example
        for await (const user of paginate(api, '/audit-log', 50)) {
            if (!EMAIL_RE.test(user.email)) {
                console.warn(`Invalid email for user ${user.id}: ${user.email}`);
            }
        }
    } catch (e) {
        if (e instanceof ApiError) {
            console.error(`API error ${e.status}: ${e.message}`);
            process.exit(1);
        }
        throw e;
    }
}

main().catch(console.error);

export { ApiClient, ApiError, UserStore, EventEmitter, paginate, clamp, escapeHtml };
