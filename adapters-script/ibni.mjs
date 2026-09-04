/**
 * ابنِ الملحقات — compiles the two TypeScript adapters of docs/tawzee.md §3.
 *
 * Rows I1 (Electron) and I2 (RPG Maker MV/MZ). `taarib-tajmee` stages the two
 * outputs and builds nothing itself, so this is the only place their compiler
 * flags exist; a second copy of them in a workflow file would be a second thing
 * to keep in step with the runtimes these files have to boot in.
 *
 *   node adapters-script/ibni.mjs [--ahdaf <cargo target dir>]
 *
 * Writes `<ahdaf>/adapters/rpgmaker/taarib.js` and
 * `<ahdaf>/adapters/electron/taarib.js`, which is where `masfufa.rs` looks.
 *
 * ## Why two compilers rather than one
 *
 * They emit for two runtimes that do not overlap.
 *
 * I2 has to be ES5 — the source header commits to it, because the oldest
 * runtime an MV game ships is NW.js 0.12, Chromium 41. esbuild cannot get
 * there: `--target=es5` fails with "Transforming const to the configured
 * target environment (\"es5\") is not supported yet" on every declaration in
 * the file, because lowering `const`/`let` to `var` is not implemented. So I2
 * is compiled by `tsc`, which is what the source header already assumed
 * ("TypeScript down-levels syntax but it does not polyfill library
 * functions"). tsc also keeps the leading `/*:` plugin block that RPG Maker's
 * Plugin Manager parses and that esbuild would drop as a non-legal comment.
 *
 * I1 has to load two ways — `require`d by the preload, and dropped into the
 * page world as a plain `<script src>` on the takeover rung — and tsc emits
 * neither shape: its UMD wrapper binds nothing global without an
 * `export as namespace`, so the page-world copy would define nothing. esbuild's
 * IIFE with a global name plus a guarded `module.exports` footer is both. tsc
 * still type-checks it, because esbuild never does.
 */

import { spawn } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { join, resolve } from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

import * as esbuild from 'esbuild';

const JIDHR = dirname(fileURLToPath(import.meta.url));

/**
 * `typescript/bin/tsc` run through this process's own node.
 *
 * Not `node_modules/.bin/tsc`: that is a shell shim on Unix and a `.cmd` on
 * Windows, and this script runs on both a maintainer's machine and a runner.
 */
const TSC = createRequire(import.meta.url).resolve('typescript/bin/tsc');

/** Runs one compiler and rejects on any non-zero exit, output passed through. */
function nadi(barnamaj, wusata) {
  return new Promise((qbul, rafd) => {
    const amaliya = spawn(barnamaj, wusata, { cwd: JIDHR, stdio: 'inherit' });
    amaliya.on('error', rafd);
    amaliya.on('close', (hala, ishara) => {
      if (hala === 0) {
        qbul();
        return;
      }
      rafd(new Error(`${barnamaj} ${wusata.join(' ')} exited ${ishara ?? hala}`));
    });
  });
}

/**
 * Row I2 — the RPG Maker MV/MZ plugin, ES5, comments and all.
 *
 * `--outDir` overrides the one in the project file so that a run pointed at a
 * different cargo target directory writes where `--ahdaf` says rather than
 * where the checked-in config defaults.
 */
async function ibni_rpgmaker(ahdaf) {
  const wajha = join(ahdaf, 'adapters/rpgmaker');
  await mkdir(wajha, { recursive: true });
  await nadi(process.execPath, [TSC, '-p', 'tsconfig.rpgmaker.json', '--outDir', wajha]);
}

/**
 * Row I1 — the Electron and NW.js renderer runtime.
 *
 * `charset: 'ascii'` deliberately: the source carries Arabic literals and the
 * page-world copy is loaded over a `file://` origin whose declared encoding
 * belongs to the game, not to us. Escaping to `\uXXXX` costs bytes and cannot
 * be misdecoded.
 *
 * es2017 rather than esnext: the optional chaining and nullish coalescing this
 * source uses are newer than the Chromium its own `adoptedStyleSheets` fallback
 * says it may land on, so they are lowered rather than shipped.
 */
async function ibni_electron(ahdaf) {
  await nadi(process.execPath, [TSC, '-p', 'tsconfig.electron.json']);
  const wajha = join(ahdaf, 'adapters/electron');
  await mkdir(wajha, { recursive: true });
  await esbuild.build({
    entryPoints: [join(JIDHR, 'electron/taarib.ts')],
    outfile: join(wajha, 'taarib.js'),
    bundle: true,
    format: 'iife',
    globalName: 'Taarib',
    target: ['es2017'],
    platform: 'browser',
    charset: 'ascii',
    legalComments: 'inline',
    logLevel: 'warning',
    sourcemap: false,
    write: true,
    footer: {
      js:
        'if (typeof module !== "undefined" && typeof module.exports !== "undefined") ' +
        '{ module.exports = Taarib; }',
    },
  });
}

function ahdaf_min_wusata(wusata) {
  if (wusata.length === 0) {
    return resolve(JIDHR, '../target');
  }
  if (wusata.length !== 2 || wusata[0] !== '--ahdaf') {
    throw new Error('usage: node ibni.mjs [--ahdaf <cargo target dir>]');
  }
  return resolve(wusata[1]);
}

try {
  const ahdaf = ahdaf_min_wusata(process.argv.slice(2));
  await ibni_rpgmaker(ahdaf);
  await ibni_electron(ahdaf);
  process.stdout.write(`ibni: I1 and I2 written under ${join(ahdaf, 'adapters')}\n`);
} catch (khata) {
  process.stderr.write(`ibni: ${khata instanceof Error ? khata.message : String(khata)}\n`);
  process.exitCode = 1;
}
