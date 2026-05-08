// tricky.js — edge cases that trip up syntax highlighters

// ── Regex vs division ambiguity ───────────────────────────────────────────────
const x = 1 / 2;                          // division
const re1 = /divide\/this/g;              // regex with escaped slash
const re2 = /^\/usr\/local/;             // regex starting with slash
let y = x / /\d+/.exec('42')[0];         // division then regex
const tagged = /* comment */ /pattern/i; // comment before regex

// ── Nested template literals ──────────────────────────────────────────────────
const a = `outer ${`inner ${`deep ${1 + 2}`} level`} end`;
const sql = `
    SELECT *
    FROM users
    WHERE name = '${`${"O'Brien"}`}'
      AND created_at > '${new Date().toISOString()}'
    LIMIT ${10 * 5}
`;

// ── ASI (automatic semicolon insertion) traps ────────────────────────────────
const obj = { a: 1 }
;[1, 2, 3].forEach(console.log)  // semicolon at start to avoid ASI trap

// ── String that looks like code ───────────────────────────────────────────────
const code = `
function fake() {
    const str = "} not a block end";
    // this comment has a " quote
    return \`nested \${template}\`;
}
`;

// ── Computed property names and weird keys ───────────────────────────────────
const key = 'dynamic';
const tricky = {
    [key]: 1,
    [`${key}_2`]: 2,
    ['string key with spaces']: 3,
    123: 4,
    true: 5,           // coerced to "true"
    [Symbol.iterator]() { return this; },
    get ['computed getter']() { return this[123]; },
    async ['async method']() { await Promise.resolve(); },
};

// ── Prototype chain and class tricks ─────────────────────────────────────────
class A {
    static #count = 0;
    #value;

    constructor(v) { this.#value = v; A.#count++; }

    // Getter with same name concept
    get value() { return this.#value; }
    set value(v) { this.#value = v; }

    static get count() { return A.#count; }

    toString() { return `A(${this.#value})`; }
    [Symbol.toPrimitive](hint) {
        if (hint === 'number') return this.#value;
        return this.toString();
    }
}

class B extends A {
    constructor(v, extra) {
        super(v);
        this.extra = extra;
    }

    // Override with super call inside complex expression
    toString() { return `B(${super.toString()}, ${this.extra})`; }
}

// ── Destructuring edge cases ──────────────────────────────────────────────────
const { a: { b: { c: deeplyNested = 'default' } = {} } = {} } = {};
const [,, third, ...rest] = [1, 2, 3, 4, 5];
function f({ x: { y = 10 } = {}, z = x => x * 2 } = {}) { return z(y); }

// ── Generators and iterators ──────────────────────────────────────────────────
function* fibonacci() {
    let [a, b] = [0, 1];
    while (true) {
        yield a;
        [a, b] = [b, a + b];
    }
}

const fib = fibonacci();
const first10 = Array.from({ length: 10 }, () => fib.next().value);

// Custom iterable
const range = {
    from: 1, to: 5,
    [Symbol.iterator]() {
        let cur = this.from;
        const last = this.to;
        return {
            next() {
                return cur <= last
                    ? { value: cur++, done: false }
                    : { value: undefined, done: true };
            },
            [Symbol.iterator]() { return this; }, // makes the iterator itself iterable
        };
    },
};

// ── Labeled statements and unusual control flow ───────────────────────────────
outer: for (let i = 0; i < 5; i++) {
    inner: for (let j = 0; j < 5; j++) {
        if (j === 2) continue outer;
        if (i === 3) break outer;
    }
}

// ── With statement (legacy, syntax highlight bait) ───────────────────────────
// with (Math) { console.log(PI, sqrt(2)); }  // commented out but present

// ── Nullish coalescing, optional chaining chains ──────────────────────────────
const val = null?.foo?.bar?.baz ?? []?.at?.(-1) ?? 'fallback';
const r = (0 || false || null || undefined || NaN || '' || 'truthy');

// ── Tagged templates with unusual tags ───────────────────────────────────────
function raw(strings, ...vals) { return strings.raw.join(''); }
const rawStr = raw`Hello\nWorld\t!`;    // \n is NOT a newline here
const css = String.raw`
    .selector > .child + .sibling ~ .general {
        content: "{ not a block }";
        background: url("data:image/svg+xml,%3Csvg%3E%3C/svg%3E");
    }
`;

// ── Numbers that look weird ───────────────────────────────────────────────────
console.log(0.1 + 0.2 === 0.3);        // false — floating point
console.log(Number.EPSILON);            // ~2.22e-16
console.log(Number.MAX_SAFE_INTEGER);   // 2^53 - 1
console.log(1e308 * 2);                 // Infinity
console.log(0 / 0);                    // NaN
console.log(-0 === 0);                 // true
console.log(Object.is(-0, 0));         // false

// ── Dynamic import and import.meta ───────────────────────────────────────────
async function load(mod) {
    const { default: m } = await import(`./modules/${mod}.js`);
    return m;
}

// ── Weird but valid variable names ───────────────────────────────────────────
const $ = 'jQuery style';
const _ = 'lodash style';
const $0 = 'zero';
const __ = '__';
const $$ = '$$';
let _ƒ_ñ_ü = 'unicode in identifier';  // valid JS

// ── try/catch/finally with return in finally (a footgun) ─────────────────────
function footgun() {
    try {
        throw new Error('oops');
    } catch (e) {
        return 'caught';    // this return is swallowed!
    } finally {
        return 'finally';   // always wins
    }
}

// ── Long line to test horizontal scroll + highlighting ───────────────────────
const LONG = { very_long_key_name_one: 1, very_long_key_name_two: 2, very_long_key_name_three: 3, very_long_key_name_four: 4, very_long_key_name_five: 5, very_long_key_name_six: 6, very_long_key_name_seven: 7 };
