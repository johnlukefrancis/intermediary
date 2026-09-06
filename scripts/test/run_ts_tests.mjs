#!/usr/bin/env node
// Path: scripts/test/run_ts_tests.mjs
// Description: Bundles every app/src/**/*_test.ts(x) with esbuild into a temp dir and runs node --test over it.

import { spawn } from 'node:child_process';
import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const sourceRoot = path.join(repoRoot, 'app', 'src');

async function collectTests(dir) {
    const entries = await fs.readdir(dir, { withFileTypes: true });
    const found = [];
    for (const entry of entries) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            found.push(...await collectTests(full));
        } else if (/_test\.tsx?$/.test(entry.name)) {
            found.push(full);
        }
    }
    return found.sort();
}

async function main() {
    const tests = await collectTests(sourceRoot);
    if (tests.length === 0) {
        process.stdout.write('No *_test.ts files under app/src.\n');
        return;
    }

    // Outside the repo on purpose: the bundle is disposable and must never reach the ledger.
    const outDir = await fs.mkdtemp(path.join(os.tmpdir(), 'intermediary-ts-tests-'));
    try {
        await build({
            entryPoints: tests,
            outdir: outDir,
            outbase: sourceRoot,
            bundle: true,
            platform: 'node',
            format: 'esm',
            target: 'node20',
            sourcemap: 'inline',
            packages: 'external',
            outExtension: { '.js': '.mjs' },
            logLevel: 'warning',
        });
        // Node resolves the bundles as ESM only via the .mjs extension: the temp dir has no package.json.
        const bundles = tests.map(test =>
            path.join(outDir, path.relative(sourceRoot, test).replace(/\.tsx?$/, '.mjs')));
        process.stdout.write(`Running ${tests.length} test file(s) from ${outDir}\n`);
        const code = await runNodeTest(bundles);
        if (code !== 0) {
            process.exitCode = code;
        }
    } finally {
        await fs.rm(outDir, { recursive: true, force: true });
    }
}

function runNodeTest(bundles) {
    return new Promise((resolve, reject) => {
        const child = spawn(process.execPath, ['--test', ...bundles], { stdio: 'inherit' });
        child.on('error', reject);
        child.on('close', (code) => { resolve(code ?? 1); });
    });
}

main().catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.stack ?? error.message : String(error)}\n`);
    process.exitCode = 1;
});
