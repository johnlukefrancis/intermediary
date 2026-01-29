#!/usr/bin/env node
// Path: scripts/fileledger/add_file_headers.mjs
// Description: Adds missing header comments (path + description) to source files using the ledger output.

import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { promises as fs } from 'node:fs';
import process from 'node:process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..', '..');
const ledgerPath = path.join(repoRoot, 'docs', 'inventory', 'file_ledger.json');

process.stdout.on('error', (error) => {
    if (error.code === 'EPIPE') {
        process.exit(0);
    }
    throw error;
});

async function main() {
    const args = process.argv.slice(2);
    const write = args.includes('--write');
    const refresh = args.includes('--refresh');
    const filters = args.filter(arg => arg !== '--write' && arg !== '--refresh');

    const ledger = await loadLedger();
    const targets = ledger.entries.filter(entry => {
        if (!filters.length) {
            return true;
        }
        return filters.some(fragment => entry.path.includes(fragment));
    });

    const missing = [];
    for (const entry of targets) {
        if (entry.path.includes('scripts/zip/Output/')) {
            continue;
        }
        const absPath = path.join(repoRoot, entry.path);
        const status = await analyzeFile(absPath, entry.path, entry.summary);
        if (!status.hasHeader) {
            missing.push({
                ...entry,
                absPath,
                original: status.original,
                sanitized: status.sanitizedBody,
                header: null,
                hasHeader: false,
                preamble: status.preamble,
                style: status.style
            });
            continue;
        }
        if (refresh && status.needsCleanup) {
            missing.push({
                ...entry,
                absPath,
                original: status.original,
                sanitized: status.sanitizedBody,
                header: status.header,
                hasHeader: true,
                preamble: status.preamble,
                style: status.style
            });
        }
    }

    if (!missing.length) {
        process.stdout.write('All inspected files already include header comments.\n');
        return;
    }

    process.stdout.write(`Found ${missing.length} file(s) missing header comments.\n`);
    process.stdout.write(missing.map(item => item.path).join('\n') + '\n');

    if (!write) {
        process.stdout.write('\nRun with --write to patch the listed files.\n');
        return;
    }

    await Promise.all(missing.map(entry => {
        if (entry.hasHeader) {
            return refreshHeader(entry);
        }
        return addHeader(entry);
    }));
    process.stdout.write('\nHeader comments injected for listed files.\n');
}

async function loadLedger() {
    let payload;
    try {
        payload = await fs.readFile(ledgerPath, 'utf8');
    } catch (error) {
        throw new Error(`Missing ledger at ${ledgerPath}. Run npm run gen:ledger first. (${error.message})`);
    }
    try {
        return JSON.parse(payload);
    } catch (error) {
        throw new Error(`Unable to parse ledger JSON: ${error.message}`);
    }
}

async function analyzeFile(absPath, relativePath, summary) {
    const original = await fs.readFile(absPath, 'utf8');
    const withoutBom = original.replace(/^\uFEFF/, '');
    const { preamble, body } = splitPreamble(withoutBom);
    const style = getCommentStyle(absPath);
    if (!style) {
        return {
            hasHeader: true,
            needsCleanup: false,
            header: null,
            sanitizedBody: body,
            preamble,
            style,
            original
        };
    }

    const headerResult = analyzeHeader(body, style);
    const sanitizedBody = stripLegacyTopComment(
        headerResult.body,
        relativePath,
        summary,
        style
    );
    return {
        hasHeader: headerResult.hasHeader,
        needsCleanup: sanitizedBody !== headerResult.body,
        header: headerResult.header,
        sanitizedBody,
        preamble,
        style,
        original
    };
}

async function addHeader({ absPath, path: relativePath, summary, sanitized, preamble, style }) {
    const header = buildHeader(relativePath, summary, style);
    const updated = `${preamble}${header}${sanitized}`;
    await fs.writeFile(absPath, updated, 'utf8');
}

async function refreshHeader({ absPath, header, sanitized, preamble }) {
    const updated = `${preamble}${header}${sanitized}`;
    await fs.writeFile(absPath, updated, 'utf8');
}

function splitPreamble(content) {
    const lines = content.split(/\r?\n/);
    if (lines[0]?.startsWith('#!')) {
        return {
            preamble: `${lines[0]}\n`,
            body: lines.slice(1).join('\n')
        };
    }
    return { preamble: '', body: content };
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

function getCommentStyle(filePath) {
    const ext = getFileExtension(filePath);
    if (['.js', '.ts', '.tsx', '.d.ts', '.mjs', '.cjs', '.mts', '.d.mts', '.d.cts', '.rs'].includes(ext)) {
        return { type: 'line', prefix: '//', suffix: '' };
    }
    if (['.py', '.sh', '.bash', '.zsh', '.ps1'].includes(ext)) {
        return { type: 'line', prefix: '#', suffix: '' };
    }
    if (['.html', '.htm'].includes(ext)) {
        return { type: 'line', prefix: '<!--', suffix: '-->' };
    }
    if (['.css', '.scss', '.sass', '.less'].includes(ext)) {
        return { type: 'block', start: '/*', linePrefix: ' * ', end: ' */' };
    }
    return null;
}

function buildHeader(relativePath, summary, style) {
    if (style.type === 'block') {
        return [
            style.start,
            `${style.linePrefix}Path: ${relativePath}`,
            `${style.linePrefix}Description: ${summary}`,
            style.end,
            '',
            ''
        ].join('\n');
    }
    const suffix = style.suffix ? ` ${style.suffix}` : '';
    return `${style.prefix} Path: ${relativePath}${suffix}\n${style.prefix} Description: ${summary}${suffix}\n\n`;
}

function analyzeHeader(body, style) {
    const lines = body.split(/\r?\n/);
    let index = 0;
    while (index < lines.length && lines[index].trim() === '') {
        index += 1;
    }
    if (index >= lines.length) {
        return { hasHeader: false, header: null, body };
    }

    if (style.type === 'block') {
        if (!lines[index].trim().startsWith(style.start)) {
            return { hasHeader: false, header: null, body };
        }
        let endIdx = index;
        while (endIdx < lines.length && !lines[endIdx].includes(style.end)) {
            endIdx += 1;
        }
        if (endIdx >= lines.length) {
            return { hasHeader: false, header: null, body };
        }
        const headerLines = lines.slice(index, endIdx + 1);
        const headerText = headerLines.join('\n');
        if (!/path\s*:/i.test(headerText) || !/description\s*:/i.test(headerText)) {
            return { hasHeader: false, header: null, body };
        }
        let headerEnd = endIdx + 1;
        if (lines[headerEnd]?.trim() === '') {
            headerEnd += 1;
        }
        return {
            hasHeader: true,
            header: ensureTrailingNewline(lines.slice(0, headerEnd).join('\n')),
            body: lines.slice(headerEnd).join('\n')
        };
    }

    const line1 = stripLineComment(lines[index], style);
    const line2 = stripLineComment(lines[index + 1] ?? '', style);
    if (
        line1?.toLowerCase().startsWith('path:') &&
        line2?.toLowerCase().startsWith('description:')
    ) {
        let headerEnd = index + 2;
        if (lines[headerEnd]?.trim() === '') {
            headerEnd += 1;
        }
        return {
            hasHeader: true,
            header: ensureTrailingNewline(lines.slice(0, headerEnd).join('\n')),
            body: lines.slice(headerEnd).join('\n')
        };
    }
    return { hasHeader: false, header: null, body };
}

function stripLineComment(value, style) {
    if (!style || style.type !== 'line') {
        return null;
    }
    const trimmed = value.trim();
    if (!trimmed.startsWith(style.prefix)) {
        return null;
    }
    let cleaned = trimmed.slice(style.prefix.length).trimStart();
    if (style.suffix && cleaned.endsWith(style.suffix)) {
        cleaned = cleaned.slice(0, -style.suffix.length);
    }
    return cleaned.trim();
}

main().catch(error => {
    process.stderr.write(`add_file_headers: ${error.message}\n`);
    process.exitCode = 1;
});

function stripLegacyTopComment(content, relativePath, summary, style) {
    if (!style || style.type !== 'line') {
        return content;
    }
    const lines = content.split(/\r?\n/);
    let index = 0;
    while (index < lines.length && lines[index].trim() === '') {
        index += 1;
    }
    if (index >= lines.length) {
        return content;
    }
    const firstComment = stripLineComment(lines[index], style);
    if (!firstComment) {
        return content;
    }
    if (firstComment.toLowerCase().startsWith('path:') || firstComment.toLowerCase().startsWith('description:')) {
        return content;
    }

    const commentBlock = [];
    let cursor = index;
    while (cursor < lines.length) {
        const stripped = stripLineComment(lines[cursor], style);
        if (stripped !== null) {
            commentBlock.push({ idx: cursor, text: lines[cursor] });
            cursor += 1;
            continue;
        }
        if (lines[cursor].trim() === '') {
            commentBlock.push({ idx: cursor, text: lines[cursor] });
            cursor += 1;
            break;
        }
        break;
    }
    if (!commentBlock.length) {
        return content;
    }

    const firstCommentText = normalizeComment(commentBlock[0]?.text ?? '', style);
    const normalizedSummary = normalizeComment(summary, style);
    const normalizedPath = normalizeComment(relativePath, style);

    const shouldDropFirstLine =
        firstCommentText &&
        (
            firstCommentText === normalizedSummary ||
            firstCommentText === normalizedPath ||
            /[\\/].+\.[a-z0-9]+$/i.test(firstCommentText)
        );

    if (!shouldDropFirstLine) {
        return content;
    }

    const updatedLines = [...lines];
    const firstIdx = commentBlock[0]?.idx;
    if (typeof firstIdx === 'number') {
        updatedLines.splice(firstIdx, 1);
    }

    while (index < updatedLines.length && updatedLines[index] && updatedLines[index].trim() === '') {
        const nextIsComment = stripLineComment(updatedLines[index + 1] ?? '', style);
        if (nextIsComment) {
            break;
        }
        if (index + 1 < updatedLines.length && updatedLines[index + 1].trim() === '') {
            updatedLines.splice(index, 1);
        } else {
            updatedLines.splice(index, 1);
            break;
        }
    }

    return updatedLines.join('\n');
}

function normalizeComment(value, style) {
    return stripLineComment(value, style)
        ?.replace(/\s+/g, ' ')
        .replace(/[–—]/g, '-')
        .trim()
        .toLowerCase() ?? '';
}

function collectHeaderEnd(lines, startIdx) {
    let idx = startIdx;
    if (idx < lines.length && lines[idx].trim() === '') {
        idx += 1;
    }
    return idx;
}

function ensureTrailingNewline(block) {
    if (block.endsWith('\n\n')) {
        return block;
    }
    if (block.endsWith('\n')) {
        return block + '\n';
    }
    return `${block}\n\n`;
}
