#!/usr/bin/env node
// Path: scripts/fileledger/gen_file_ledger.mjs
// Description: Generates human+machine file ledgers for Intermediary code sources.

import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { promises as fs } from 'node:fs';
import process from 'node:process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..', '..');
const defaultRootNames = [
    'src-tauri',
    'app',
    'agent',
    'scripts'
];
const args = process.argv.slice(2);
const requestedRoots = args.filter(arg => !arg.startsWith('--'));
const rootNames = requestedRoots.length ? requestedRoots : defaultRootNames;
const ledgerRoots = rootNames.map(resolveRoot);
let scopeLabel = '';
const mdLedgerPath = path.join(repoRoot, 'docs', 'inventory', 'file_ledger.md');
const jsonLedgerPath = path.join(repoRoot, 'docs', 'inventory', 'file_ledger.json');
const allowedOutputs = new Set([mdLedgerPath, jsonLedgerPath]);

const excludedDirNames = new Set([
    'node_modules',
    '.git',
    'dist',
    'build',
    '.cache',
    'coverage',
    'tmp',
    'temp',
    '.venv',
    '__pycache__',
    'assets',
    'Assets',
    'Vendor',
    'target',
    'Output'
]);

const skippedDirFragments = ['Vendor', 'ThirdParty', 'External'];

const trackedExtensions = new Set([
    '.js',
    '.ts',
    '.tsx',
    '.d.ts',
    '.mjs',
    '.cjs',
    '.mts',
    '.d.mts',
    '.d.cts',
    '.rs',
    '.py',
    '.css',
    '.scss',
    '.html'
]);

scopeLabel = buildScopeLabel(rootNames);

const suffixRoleMap = new Map([
    ['Manager', 'manager'],
    ['Controller', 'controller'],
    ['Renderer', 'renderer'],
    ['Panel', 'panel'],
    ['Store', 'state store'],
    ['Flags', 'flag defaults'],
    ['Bridge', 'bridge layer'],
    ['State', 'state container'],
    ['System', 'system'],
    ['Service', 'service'],
    ['Runner', 'runner'],
    ['Harness', 'harness'],
    ['Step', 'step'],
    ['Steps', 'steps'],
    ['Queue', 'queue'],
    ['Schema', 'schema'],
    ['Config', 'configuration'],
    ['Map', 'mapping'],
    ['Utils', 'utility helpers'],
    ['Hooks', 'hooks'],
    ['Registry', 'registry'],
    ['Bus', 'bus'],
    ['Adapter', 'adapter'],
    ['Entry', 'entry'],
    ['Main', 'entry point'],
    ['Index', 'entry point'],
    ['Client', 'client'],
    ['Server', 'server'],
    ['Model', 'model'],
    ['View', 'view']
]);

const segmentRoleMap = new Map([
    ['src-tauri', 'tauri backend'],
    ['app', 'frontend'],
    ['agent', 'wsl agent'],
    ['scripts', 'scripts'],
    ['staging', 'staging system'],
    ['ipc', 'ipc protocol'],
    ['watcher', 'file watcher'],
    ['bundle', 'bundle builder'],
    ['config', 'configuration'],
    ['paths', 'path helpers'],
    ['debug', 'debug'],
    ['settings', 'settings'],
    ['core', 'core'],
    ['io', 'io']
]);

async function main() {
    validateEnvironment();

    const discovery = {
        scanned: 0,
        included: 0,
        skipped: 0,
        entries: [],
        extensionCounts: new Map(),
        directoryCounts: new Map(),
        tbdDirectories: new Map()
    };

    for (const root of ledgerRoots) {
        await walk(root, discovery);
    }

    discovery.entries.sort((a, b) => {
        const aa = a.path.toLowerCase();
        const bb = b.path.toLowerCase();
        if (aa === bb) {
            return a.path.localeCompare(b.path);
        }
        return aa.localeCompare(bb);
    });

    const nonTbdCount = discovery.entries.filter(({ summary }) => summary !== 'TBD').length;
    const coverage = discovery.entries.length === 0
        ? 100
        : (nonTbdCount / discovery.entries.length) * 100;

    if (discovery.entries.length !== discovery.included) {
        throw new Error('Internal accounting mismatch: included count differs from entry list.');
    }

    await writeLedgers(discovery.entries);
    emitReport({
        scanned: discovery.scanned,
        included: discovery.included,
        skipped: discovery.skipped,
        nonTbdCount,
        coverage,
        extensionCounts: discovery.extensionCounts,
        directoryCounts: discovery.directoryCounts,
        tbdDirectories: discovery.tbdDirectories,
        entries: discovery.entries
    });

    if (coverage < 95) {
        throw new Error(`Coverage ${coverage.toFixed(2)}% below threshold (95%).`);
    }
}

function validateEnvironment() {
    const touching = Array.from(allowedOutputs);
    if (touching.length !== 2) {
        throw new Error('Unexpected output target set sizing.');
    }
}

function resolveRoot(rootName) {
    const resolved = path.isAbsolute(rootName)
        ? rootName
        : path.join(repoRoot, rootName);
    if (!resolved.startsWith(repoRoot)) {
        throw new Error(`Ledger root must live inside repo: ${rootName}`);
    }
    return resolved;
}

function buildScopeLabel(roots) {
    const extList = Array.from(trackedExtensions).sort().join(', ');
    const rootList = roots.length ? roots.join(', ') : 'repo';
    return `${rootList} (extensions: ${extList})`;
}

function lineCommentPrefixes(ext) {
    const lower = ext.toLowerCase();
    if (['.py', '.sh', '.bash', '.zsh', '.ps1'].includes(lower)) {
        return ['#'];
    }
    if (['.html', '.htm'].includes(lower)) {
        return ['<!--'];
    }
    return ['//'];
}

function escapeRegExp(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function getFileExtension(filePath) {
    const normalized = filePath.toLowerCase();
    if (normalized.endsWith('.d.ts')) {
        return '.d.ts';
    }
    if (normalized.endsWith('.d.mts')) {
        return '.d.mts';
    }
    if (normalized.endsWith('.d.cts')) {
        return '.d.cts';
    }
    return path.extname(normalized);
}

async function walk(currentDir, discovery) {
    const dirEntries = await fs.readdir(currentDir, { withFileTypes: true });

    for (const entry of dirEntries) {
        const entryPath = path.join(currentDir, entry.name);
        const relativePath = path.relative(repoRoot, entryPath);

        if (entry.isDirectory()) {
            if (shouldSkipDir(entry, entryPath)) {
                continue;
            }
            await walk(entryPath, discovery);
            continue;
        }

        if (!entry.isFile()) {
            continue;
        }

        const ext = getFileExtension(entryPath);
        if (!trackedExtensions.has(ext)) {
            continue;
        }

        discovery.scanned += 1;

        if (shouldSkipFile(entryPath)) {
            discovery.skipped += 1;
            continue;
        }

        const summary = await summarizeFile(entryPath, ext);
        discovery.included += 1;
        const normalizedPath = relativePath.split(path.sep).join('/');
        discovery.entries.push({ path: normalizedPath, summary });

        discovery.extensionCounts.set(ext, (discovery.extensionCounts.get(ext) || 0) + 1);

        const dirName = path.dirname(normalizedPath);
        discovery.directoryCounts.set(dirName, (discovery.directoryCounts.get(dirName) || 0) + 1);

        if (summary === 'TBD') {
            discovery.tbdDirectories.set(dirName, (discovery.tbdDirectories.get(dirName) || 0) + 1);
        }
    }
}

function shouldSkipDir(entry, fullPath) {
    if (excludedDirNames.has(entry.name)) {
        return true;
    }
    const normalized = fullPath.split(path.sep).join('/');
    return skippedDirFragments.some(fragment => normalized.includes(`/${fragment}/`));
}

function shouldSkipFile(fullPath) {
    const normalized = fullPath.split(path.sep).join('/');
    return skippedDirFragments.some(fragment => normalized.includes(`/${fragment}/`));
}

async function summarizeFile(filePath, ext) {
    const content = await fs.readFile(filePath, 'utf8');
    const baseName = path.basename(filePath);
    const commentSummary = extractCommentSummary(content, baseName, ext);
    if (commentSummary) {
        return commentSummary;
    }

    const defaultExport = extractDefaultExportName(content);
    if (defaultExport) {
        const contextual = buildContextualSummary(defaultExport, filePath);
        if (contextual) {
            return contextual;
        }
    }

    const pathSummary = buildPathSummary(filePath, ext);
    if (pathSummary) {
        return pathSummary;
    }

    return 'TBD';
}

function extractCommentSummary(content, baseName, ext) {
    const linePrefixes = lineCommentPrefixes(ext);
    const linePrefixPattern = linePrefixes.length
        ? new RegExp(`^\\s*(${linePrefixes.map(escapeRegExp).join('|')})\\s?`)
        : null;
    const leadingMatch = (() => {
        const lines = content.split(/\r?\n/);
        let index = 0;
        while (index < lines.length && lines[index].trim() === '') {
            index += 1;
        }
        if (index < lines.length && lines[index].trim().startsWith('#!')) {
            index += 1;
        }
        while (index < lines.length && lines[index].trim() === '') {
            index += 1;
        }
        if (index >= lines.length) {
            return null;
        }
        const line = lines[index].trim();
        if (line.startsWith('/*')) {
            const sliced = lines.slice(index).join('\n');
            const end = sliced.indexOf('*/');
            if (end === -1) {
                return null;
            }
            return sliced.slice(0, end + 2);
        }
        const isLineComment = (value) => {
            const trimmed = value.trim();
            return linePrefixes.some(prefix => trimmed.startsWith(prefix));
        };
        if (isLineComment(line)) {
            const commentLines = [];
            for (let cursor = index; cursor < lines.length; cursor += 1) {
                const current = lines[cursor];
                if (isLineComment(current)) {
                    commentLines.push(current);
                    continue;
                }
                break;
            }
            return commentLines.join('\n');
        }
        return null;
    })();

    if (!leadingMatch) {
        return null;
    }

    const cleanedLines = leadingMatch
        .replace(/^\/\*+/, '')
        .replace(/\*+\/$/, '')
        .split(/\r?\n/)
        .map(line => {
            let cleaned = line;
            if (linePrefixPattern) {
                cleaned = cleaned.replace(linePrefixPattern, '');
            }
            cleaned = cleaned.replace(/^\s*\*+\s?/, '').replace(/\s*-->$/, '');
            return cleaned.trim();
        })
        .filter(Boolean);

    if (cleanedLines.length === 0) {
        return null;
    }

    const filteredLines = cleanedLines
    .filter(line => !/^path:\s*/i.test(line))
    .map(line => line.replace(/^(description|summary|file):\s*/i, '').trim())
    .filter(line => !/^@ts-(check|nocheck|expect-error|ignore)/i.test(line))
    .filter(Boolean);

    const dedupedLines = [];
    const seenLines = new Set();
    for (const line of filteredLines) {
        const normalized = line.toLowerCase();
        if (seenLines.has(normalized)) {
            continue;
        }
        seenLines.add(normalized);
        dedupedLines.push(line);
    }

    if (dedupedLines.length === 0) {
        return null;
    }

    const directiveLine = dedupedLines.find(line => /@file|@module/i.test(line));
    let summary = directiveLine
        ? directiveLine.replace(/.*@(file|module)\s*/i, '').trim()
        : dedupedLines.join(' ');

    summary = summary.replace(/\s+/g, ' ').trim();

    if (!summary) {
        return null;
    }

    const baseLower = baseName.replace(/\.[^.]+$/, '').toLowerCase();
    const summaryLower = summary.toLowerCase();
    const disallowed = new Set([baseLower, `${baseLower}.js`, `${baseLower} module`, `${baseLower} file`]);
    if (disallowed.has(summaryLower) || summaryLower.length < 4) {
        return null;
    }

    return clampSummary(summary);
}

function extractDefaultExportName(content) {
    const defaultClass = content.match(/export\s+default\s+class\s+([A-Za-z0-9_]+)/);
    if (defaultClass) {
        return defaultClass[1];
    }
    const defaultFunction = content.match(/export\s+default\s+function\s+([A-Za-z0-9_]+)/);
    if (defaultFunction) {
        return defaultFunction[1];
    }
    const namedDefault = content.match(/export\s+default\s+([A-Za-z0-9_]+)/);
    if (namedDefault) {
        return namedDefault[1];
    }
    const aliasDefault = content.match(/export\s*{\s*([A-Za-z0-9_]+)\s+as\s+default\s*}/);
    if (aliasDefault) {
        return aliasDefault[1];
    }
    return null;
}

function buildContextualSummary(identifier, filePath) {
    const words = splitWords(identifier);
    if (words.length === 0) {
        return null;
    }
    const role = suffixRoleMap.get(words[words.length - 1]) || 'module';
    const context = deriveContextFromPath(filePath);
    const summary = composeSummary(words, context, role);
    return summary ? clampSummary(summary) : null;
}

function buildPathSummary(filePath, ext) {
    const baseName = path.basename(filePath, ext);
    const words = splitWords(baseName);
    const context = deriveContextFromPath(filePath);
    const role = suffixRoleMap.get(words[words.length - 1]) || 'module';
    const summary = composeSummary(words.length ? words : [baseName], context, role);
    if (!summary || summary.toLowerCase() === 'module') {
        return null;
    }
    return clampSummary(summary);
}

function deriveContextFromPath(filePath) {
    const baseRoot = ledgerRoots.find(root => filePath.startsWith(root)) || repoRoot;
    const relative = path.relative(baseRoot, filePath);
    const segments = relative.split(path.sep);
    segments.pop(); // remove filename
    const phrases = [];

    for (const segment of segments) {
        const clean = segment.replace(/\.[^.]+$/, '');
        const mapped = segmentRoleMap.get(clean);
        if (mapped && !phrases.includes(mapped)) {
            phrases.push(mapped);
        }
    }

    return phrases;
}

function splitWords(identifier) {
    return identifier
        .replace(/[-_]+/g, ' ')
        .replace(/([a-z])([A-Z])/g, '$1 $2')
        .split(/\s+/)
        .map(word => word.trim())
        .filter(Boolean)
        .map(word => word.replace(/^[A-Z]{2,}$/, match => match[0] + match.slice(1).toLowerCase()));
}

function composeSummary(nameWords, contextPhrases, role) {
    const lowerName = nameWords.map(word => word.toLowerCase());
    const contextWords = contextPhrases
        .flatMap(phrase => phrase.split(/\s+/))
        .map(word => word.toLowerCase())
        .filter(Boolean);
    const roleWords = role.split(/\s+/).map(word => word.toLowerCase());
    const ordered = [...lowerName, ...contextWords, ...roleWords];
    const deduped = [];
    const seen = new Set();
    for (const word of ordered) {
        if (!word) {
            continue;
        }
        if (seen.has(word)) {
            continue;
        }
        seen.add(word);
        deduped.push(word);
    }
    return deduped.join(' ').trim();
}

async function writeLedgers(entries) {
    await ensureOnlyAllowedTargets();

    const scope = scopeLabel;

    const mdHeader = [
        '# File Ledger',
        '',
        `Scope: ${scope}`,
        '',
        '```text'
    ];

    const mdLines = entries.map(({ path, summary }) => `${path} - ${summary}`);
    const mdContent = [...mdHeader, ...mdLines, '```', ''].join('\n');

    const jsonContent = JSON.stringify({
        scope,
        entries
    }, null, 2) + '\n';

    await fs.writeFile(mdLedgerPath, mdContent, 'utf8');
    await fs.writeFile(jsonLedgerPath, jsonContent, 'utf8');
}

async function ensureOnlyAllowedTargets() {
    const targets = [mdLedgerPath, jsonLedgerPath];
    for (const target of targets) {
        if (!allowedOutputs.has(target)) {
            throw new Error(`Refusing to write unexpected target: ${target}`);
        }
        await fs.mkdir(path.dirname(target), { recursive: true });
    }
}

function emitReport(report) {
    const {
        scanned,
        included,
        skipped,
        nonTbdCount,
        coverage,
        extensionCounts,
        directoryCounts,
        tbdDirectories,
        entries
    } = report;

    const coverageBlock = [
        '=== Ledger Coverage ===',
        `total scanned: ${scanned}`,
        `included: ${included}`,
        `skipped: ${skipped}`,
        `non-TBD: ${nonTbdCount}`,
        `coverage: ${coverage.toFixed(2)}%`,
        ''
    ];

    const extensionLines = Array.from(extensionCounts.entries())
        .sort((a, b) => b[1] - a[1])
        .map(([ext, count]) => `${ext}: ${count}`);
    if (extensionLines.length === 0) {
        extensionLines.push('none');
    }

    const directories = Array.from(directoryCounts.entries())
        .sort((a, b) => {
            if (b[1] === a[1]) {
                return a[0].localeCompare(b[0]);
            }
            return b[1] - a[1];
        })
        .slice(0, 10)
        .map(([dir, count]) => `${dir} (${count})`);

    const tbdHotspots = Array.from(tbdDirectories.entries())
        .sort((a, b) => {
            if (b[1] === a[1]) {
                return a[0].localeCompare(b[0]);
            }
            return b[1] - a[1];
        })
        .map(([dir, count]) => `${dir} (${count})`);

    const reportLines = [
        ...coverageBlock,
        '=== Extension Counts ===',
        ...extensionLines,
        '',
        '=== Top Directories ===',
        ...(directories.length ? directories : ['none']),
        '',
        '=== TBD Hotspots ===',
        ...(tbdHotspots.length ? tbdHotspots : ['none']),
        '',
        '=== Ledger Sample (20 lines) ===',
        ...entries.slice(0, 20).map(({ path, summary }) => `${path} - ${summary}`),
        ''
    ];

    process.stdout.write(reportLines.join('\n'));
}

function clampSummary(summary) {
    let clean = summary.replace(/\s+/g, ' ').trim();
    clean = clean.replace(/[–—]/g, '-').replace(/[“”]/g, '"').replace(/[‘’]/g, '\'');
    clean = clean.replace(/[^\x20-\x7E]/g, '');
    if (clean.length <= 120) {
        return clean;
    }
    return `${clean.slice(0, 117).trimEnd()}...`;
}

main().catch(error => {
    process.stderr.write(`gen_file_ledger: ${error.message}\n`);
    process.exitCode = 1;
});
