// Generates parity.json from break_eternity.js.
// Run from the fixtures directory:
//   cd tests/fixtures && npm install break_eternity.js@2.1.3 && node generate.mjs
//
// The pinned version matters: the fixture documents parity with a specific
// upstream release. Bump it deliberately and note it in CHANGELOG.md.
//
// The package uses CJS exports so we use createRequire to load it.
import { createRequire } from 'module';
import { writeFileSync, readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

const pkg = JSON.parse(readFileSync(path.join(__dirname, 'node_modules/break_eternity.js/package.json'), 'utf8'));
const EXPECTED_VERSION = '2.1.3';
if (pkg.version !== EXPECTED_VERSION) {
  process.stderr.write(`break_eternity.js ${pkg.version} installed, expected ${EXPECTED_VERSION}; run npm install break_eternity.js@${EXPECTED_VERSION}\n`);
  process.exit(1);
}

const Decimal = require(path.join(__dirname, 'node_modules/break_eternity.js/dist/break_eternity.js'));

const cases = [];

// Helper: stringify a Decimal at full precision.
const dstr = (d) => d.toString();

// Results the string-based fixture cannot express (the Rust side reports these
// as errors, which parity.rs treats as "JS produced no value").
const unrepresentable = (r) => {
  const rs = dstr(r);
  return rs === 'NaN' || rs === 'Infinity' || rs === '-Infinity' || (r.sign === 0 && isNaN(r.mag));
};

// Layer-0 values across many magnitudes (positive and negative).
// Note: '5e-324' and '-5e-324' are the IEEE 754 subnormal minimum. The JS
// implementation adds a precision correction (+3.27e-16) that the Rust
// implementation does not replicate. Excluded from the fixture to avoid
// systematic false failures on this JS-specific artifact.
const layer0 = [
  '0', '1', '-1', '2', '-2', '10', '-10', '100', '0.5', '0.1', '1e-10', '1e-300',
  '1e10', '-1e10', '1e15', '9e15', '1e100', '1e308',
  '1.5', '-1.5', '2.5', '-2.5', '3.7', '0.25', '0.9', '1.15', '7', '-7', '123456.789',
];

// Layer-1 and beyond.
const highLayer = [
  '1e1000', '1e1e15', 'ee15', 'ee100', 'ee1000', 'eee15',
  '1e1e1000', '(e^4)15', '(e^10)100',
  '-1e1000', '-ee100', '1e-1000', 'ee-100',
];

const allInputs = [...layer0, ...highLayer];

// Unary operations on each input.
const unaryOps = {
  abs: (d) => d.abs(),
  neg: (d) => d.neg(),
  sqrt: (d) => d.sqrt(),
  cbrt: (d) => d.cbrt(),
  ln: (d) => d.ln(),
  log10: (d) => d.log10(),
  log2: (d) => d.log2(),
  exp: (d) => d.exp(),
  gamma: (d) => d.gamma(),
  factorial: (d) => d.factorial(),
  recip: (d) => d.recip(),
  floor: (d) => d.floor(),
  ceil: (d) => d.ceil(),
  round: (d) => d.round(),
  trunc: (d) => d.trunc(),
  tetrate2: (d) => d.tetrate(2),
  // Fractional heights exercise the analytic critical-section path. The
  // _linear variant lets us parity-test both modes against the JS reference.
  tetrate2_5: (d) => d.tetrate(2.5, undefined, false),
  tetrate2_5_linear: (d) => d.tetrate(2.5, undefined, true),
  tetrate1_5: (d) => d.tetrate(1.5, undefined, false),
  tetrate_neg0_5: (d) => d.tetrate(-0.5, undefined, false),
  tetrate_inf: (d) => d.tetrate(Infinity),
  slog10: (d) => d.slog(10),
  slog2: (d) => d.slog(2),
  slog10_linear: (d) => d.slog(10, 100, true),
  ssqrt: (d) => d.ssqrt(),
  sroot3: (d) => d.linear_sroot(3),
  pentate2: (d) => d.pentate(2),
  penta_log10: (d) => d.penta_log(10),
  lambertw: (d) => d.lambertw(),
  lambertw_neg1: (d) => d.lambertw(false),
  sinh: (d) => d.sinh(),
  cosh: (d) => d.cosh(),
  tanh: (d) => d.tanh(),
  asinh: (d) => d.asinh(),
  acosh: (d) => d.acosh(),
  atanh: (d) => d.atanh(),
  sin: (d) => d.sin(),
  cos: (d) => d.cos(),
  atan: (d) => d.atan(),
  layeradd10_1: (d) => d.layeradd10(1),
  layeradd10_neg1: (d) => d.layeradd10(-1),
  layeradd10_0_5: (d) => d.layeradd10(0.5),
  iteratedlog10_2: (d) => d.iteratedlog(10, 2),
  iteratedlog10_1_5: (d) => d.iteratedlog(10, 1.5),
};

for (const aStr of allInputs) {
  let a;
  try {
    a = Decimal.fromString(aStr);
  } catch (e) {
    process.stderr.write(`Skipping unparseable input: ${aStr}\n`);
    continue;
  }
  for (const [opName, opFn] of Object.entries(unaryOps)) {
    // Upstream penta_log is documented as "INCREDIBLY slow on numbers <= -1" (it never
    // returns for -1); the Rust port bails out, so there is nothing to compare.
    if (opName === 'penta_log10' && a.lte(-1)) continue;
    try {
      const r = opFn(a);
      if (unrepresentable(r)) continue;
      cases.push({ op: opName, a: aStr, result: dstr(r) });
    } catch (e) {
      // JS variant threw; skip.
    }
  }
}

// Binary operations: pair every layer0 value with every layer0 value.
const binaryOps = {
  add: (a, b) => a.add(b),
  sub: (a, b) => a.sub(b),
  mul: (a, b) => a.mul(b),
  div: (a, b) => a.div(b),
  pow: (a, b) => a.pow(b),
  root: (a, b) => a.root(b),
  log: (a, b) => a.log(b),
  mod: (a, b) => a.mod(b),
  mod_floored: (a, b) => a.mod(b, true),
  tetrate_payload: (a, b) => a.tetrate(2, b),
  layeradd_base: (a, b) => a.layeradd(1, b),
};

// Binary ops whose JS result is well-defined but where the Rust port
// deliberately answers differently. `pow` of a layer-0 pair goes through an
// exact powf in Rust, so integer powers land on the exact integer where JS
// carries a few ulps of log-domain error; both agree within tolerance.
for (const aStr of layer0) {
  for (const bStr of layer0) {
    let a, b;
    try {
      a = Decimal.fromString(aStr);
      b = Decimal.fromString(bStr);
    } catch (e) { continue; }
    for (const [opName, opFn] of Object.entries(binaryOps)) {
      try {
        const r = opFn(a, b);
        if (unrepresentable(r)) continue;
        cases.push({ op: opName, a: aStr, b: bStr, result: dstr(r) });
      } catch (e) { /* skip */ }
    }
  }
}

// Cross-product: high-layer vs low-layer.
for (const aStr of highLayer) {
  for (const bStr of layer0.slice(0, 12)) {
    let a, b;
    try {
      a = Decimal.fromString(aStr);
      b = Decimal.fromString(bStr);
    } catch (e) { continue; }
    for (const [opName, opFn] of Object.entries(binaryOps)) {
      try {
        const r = opFn(a, b);
        if (unrepresentable(r)) continue;
        cases.push({ op: opName, a: aStr, b: bStr, result: dstr(r) });
        const r2 = opFn(b, a);
        if (!unrepresentable(r2)) cases.push({ op: opName, a: bStr, b: aStr, result: dstr(r2) });
      } catch (e) { /* skip */ }
    }
  }
}

// String parsing: every accepted notation must produce the same value.
const parseInputs = [
  '1', '-1', '0', '1e5', '1.5e-7', '1e1000', '1e-400', 'e3', '-e3', 'ee3', 'eee3', '1e1e3',
  '2ee3', '(e^3)2', '-(e^7)15.5', '(e^100)15000000000', '(e^1.5)5', '(e^-1)5',
  '10^3', '2^10', '10^-1', '2^^3', '2^^2;3', '2^^^2', '2^^^2;2', '10^^2.5', '1.5^^2.5',
  '3pt2', '3 PT 2', '3 PT (2)', '-3pt2', '2p3', '1PT3', '2f3', 'f2', 'F2',
  '1,000', '  42  ', '+5', '.5', '5.', '1E5', '0e5', '2.47e-324', '1e-310', '9e15', '1e21',
];
for (const s of parseInputs) {
  try {
    const r = Decimal.fromString(s);
    if (unrepresentable(r)) continue;
    cases.push({ op: 'parse', a: s, result: dstr(r) });
  } catch (e) { /* skip */ }
}

// Game helpers (4-ary): {resources, start, ratio/add, owned}.
const seriesInputs = [
  ['100', '10', '1.5', '0'], ['1e500', '1', '1.15', '0'], ['1e50', '1e10', '1.07', '30'],
  ['52', '10', '2', '0'], ['1e20', '5', '3', '100'], ['1e300', '1e100', '1.0001', '0'],
];
for (const [res, start, step, owned] of seriesInputs) {
  const push = (op, r) => { if (!unrepresentable(r)) cases.push({ op, a: res, b: start, c: step, d: owned, result: dstr(r) }); };
  try { push('afford_geometric', Decimal.affordGeometricSeries(res, start, step, owned)); } catch (e) {}
  try { push('sum_geometric', Decimal.sumGeometricSeries(res, start, step, owned)); } catch (e) {}
  try { push('afford_arithmetic', Decimal.affordArithmeticSeries(res, start, step, owned)); } catch (e) {}
  try { push('sum_arithmetic', Decimal.sumArithmeticSeries(res, start, step, owned)); } catch (e) {}
}

process.stderr.write(`Generated ${cases.length} parity cases against break_eternity.js ${pkg.version}.\n`);
writeFileSync(path.join(__dirname, 'parity.json'), JSON.stringify(cases, null, 2));
process.stderr.write(`Written to tests/fixtures/parity.json\n`);
