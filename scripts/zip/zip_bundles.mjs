// Path: scripts/zip/zip_bundles.mjs
// Description: Builds timestamped Intermediary zip bundles for ChatGPT context.

import { execSync, spawnSync } from 'node:child_process';
import { promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);
const scriptRoot = path.dirname(scriptPath);
const repoRoot = path.resolve(scriptRoot, '..', '..');
const outputRoot = path.join(scriptRoot, 'output');

function formatTimestamp(date) {
  const pad = (value) => String(value).padStart(2, '0');
  return `${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}_${pad(
    date.getHours(),
  )}${pad(date.getMinutes())}${pad(date.getSeconds())}`;
}

function getGitShortSha() {
  try {
    return execSync('git rev-parse --short HEAD', {
      cwd: repoRoot,
      stdio: ['ignore', 'pipe', 'ignore'],
    })
      .toString()
      .trim();
  } catch {
    return '';
  }
}

function isWsl() {
  if (process.platform !== 'linux') {
    return false;
  }
  if (process.env.WSL_DISTRO_NAME || process.env.WSL_INTEROP) {
    return true;
  }
  try {
    const version = execSync('cat /proc/version', { stdio: ['ignore', 'pipe', 'ignore'] })
      .toString()
      .toLowerCase();
    return version.includes('microsoft');
  } catch {
    return false;
  }
}

function tryCommand(cmd, args) {
  const result = spawnSync(cmd, args, { stdio: 'ignore' });
  return result.status === 0;
}

function resolvePowerShell() {
  const candidates = ['pwsh', 'powershell', 'powershell.exe'];
  for (const candidate of candidates) {
    if (tryCommand(candidate, ['-NoProfile', '-Command', '$PSVersionTable.PSVersion'])) {
      return candidate;
    }
  }
  return null;
}

function toWindowsPath(wslPath) {
  return execSync(`wslpath -w "${wslPath.replace(/"/g, '\\"')}"`, {
    stdio: ['ignore', 'pipe', 'ignore'],
  })
    .toString()
    .trim();
}

async function ensureDir(dirPath) {
  await fs.mkdir(dirPath, { recursive: true });
}

async function copyRecursive(source, destination) {
  const stats = await fs.stat(source);
  if (stats.isDirectory()) {
    await ensureDir(destination);
    const entries = await fs.readdir(source);
    for (const entry of entries) {
      await copyRecursive(path.join(source, entry), path.join(destination, entry));
    }
    return;
  }
  await ensureDir(path.dirname(destination));
  await fs.copyFile(source, destination);
}

function shouldSkipPath(relativePath) {
  const normalizedPath = relativePath.replace(/\\/g, '/');
  if (normalizedPath === 'logs' || normalizedPath.startsWith('logs/')) {
    return false;
  }
  if (normalizedPath.endsWith('/.gitkeep')) {
    return false;
  }
  const skipPatterns = [
    'target/',
    'node_modules/',
    'dist/',
    '.git/',
    'scripts/zip/output/',
    'assets/',
    // App icons (binary, not needed for context)
    'src-tauri/icons/',
    // Test report images (binary outputs)
    'docs/reports/bundles/',
    // WSL agent virtual environment (future)
    'agent/.venv/',
    'agent/__pycache__/',
    // File extensions
    '*.log',
    '*.pyc',
  ];
  return skipPatterns.some(pattern => {
    if (pattern.startsWith('*')) {
      return relativePath.endsWith(pattern.slice(1));
    }
    return relativePath.includes(pattern);
  });
}

async function copyWithFilter(source, destination, relativeRoot) {
  const stats = await fs.stat(source);
  if (stats.isDirectory()) {
    await ensureDir(destination);
    const entries = await fs.readdir(source);
    for (const entry of entries) {
      const nextSource = path.join(source, entry);
      const nextRelative = path.join(relativeRoot, entry).replace(/\\/g, '/');
      if (shouldSkipPath(nextRelative)) {
        continue;
      }
      const nextDestination = path.join(destination, entry);
      await copyWithFilter(nextSource, nextDestination, nextRelative);
    }
    return;
  }
  await ensureDir(path.dirname(destination));
  await fs.copyFile(source, destination);
}

async function listFiles(rootDir) {
  const entries = await fs.readdir(rootDir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const fullPath = path.join(rootDir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await listFiles(fullPath)));
    } else if (entry.isFile()) {
      files.push(fullPath);
    }
  }
  return files.sort();
}

async function listRootFiles(rootDir) {
  const entries = await fs.readdir(rootDir, { withFileTypes: true });
  return entries
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name)
    .filter((name) => !shouldSkipPath(name))
    .sort();
}

async function getTotalSize(files) {
  const statResults = await Promise.all(
    files.map(async (filePath) => {
      try {
        const stats = await fs.stat(filePath);
        return stats.size;
      } catch {
        return 0;
      }
    }),
  );
  return statResults.reduce((sum, size) => sum + size, 0);
}

async function writeManifest(stagingRoot, bundleName, sha, contentsRootLabel = 'Intermediary/') {
  const generatedAt = new Date().toISOString();
  const shaLabel = sha || 'unavailable';
  const manifestPath = path.join(stagingRoot, '_MANIFEST.md');
  const header = [
    '# Intermediary Bundle Manifest',
    '',
    `Bundle: ${bundleName}`,
    `Generated: ${generatedAt}`,
    `Git SHA: ${shaLabel}`,
    `Contents root: ${contentsRootLabel}`,
    '',
    'Files:',
  ];
  await fs.writeFile(manifestPath, `${header.join('\n')}\n`, 'utf8');

  const files = await listFiles(stagingRoot);
  const lines = files.map((filePath) => {
    const relative = path.relative(stagingRoot, filePath).replace(/\\/g, '/');
    return `- ${relative}`;
  });
  await fs.appendFile(manifestPath, `${lines.join('\n')}\n`, 'utf8');
}

function compressArchive(stagingRoot, zipPath) {
  const psCommand = resolvePowerShell();
  if (!psCommand) {
    console.error('PowerShell not found. Install PowerShell 7 (pwsh) or make powershell.exe available.');
    throw new Error('PowerShell not found.');
  }

  const needsWindowsPath = isWsl() && psCommand.toLowerCase().endsWith('.exe');
  const sourceRoot = needsWindowsPath ? toWindowsPath(stagingRoot) : stagingRoot;
  const destination = needsWindowsPath ? toWindowsPath(zipPath) : zipPath;
  const escapePowerShellLiteral = (value) => `'${String(value).replace(/'/g, "''")}'`;
  const command = [
    'Set-StrictMode -Version Latest;',
    "$ErrorActionPreference = 'Stop';",
    'Add-Type -AssemblyName System.IO.Compression;',
    'Add-Type -AssemblyName System.IO.Compression.FileSystem;',
    `$src = ${escapePowerShellLiteral(sourceRoot)};`,
    `$dest = ${escapePowerShellLiteral(destination)};`,
    'if (Test-Path -LiteralPath $dest) { Remove-Item -LiteralPath $dest -Force; }',
    '$zip = [System.IO.Compression.ZipFile]::Open($dest, [System.IO.Compression.ZipArchiveMode]::Create);',
    'try {',
    '  $files = Get-ChildItem -LiteralPath $src -Recurse -File;',
    "  $srcTrim = $src -replace '[\\\\/]+$','';",
    '  $prefix = $srcTrim + [IO.Path]::DirectorySeparatorChar;',
    '  foreach ($file in $files) {',
    '    $full = $file.FullName;',
    '    if ($full.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {',
    '      $relative = $full.Substring($prefix.Length);',
    '    } else {',
    '      $relative = $file.Name;',
    '    }',
    "    $entryName = $relative -replace '\\\\','/';",
    '    [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(' +
      '$zip, $file.FullName, $entryName, [System.IO.Compression.CompressionLevel]::Optimal' +
      ') | Out-Null;',
    '  }',
    '} finally {',
    '  $zip.Dispose();',
    '}',
  ].join(' ');

  const args = [
    '-NoProfile',
    '-ExecutionPolicy',
    'Bypass',
    '-Command',
    command,
  ];

  const result = spawnSync(psCommand, args, { stdio: 'inherit' });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`Compress-Archive failed with exit code ${result.status}.`);
  }
}

async function removePath(targetPath) {
  try {
    await fs.rm(targetPath, { recursive: true, force: true });
  } catch {
    // Ignore cleanup errors.
  }
}

async function copyEntry(entry, stagingRoot) {
  const sourcePath = path.join(repoRoot, entry.source);
  const destinationPath = path.join(stagingRoot, entry.dest ?? entry.source);
  if (entry.filter) {
    await copyWithFilter(sourcePath, destinationPath, entry.dest ?? entry.source);
  } else {
    await copyRecursive(sourcePath, destinationPath);
  }
}

async function copyFile(source, stagingRoot) {
  const sourcePath = path.join(repoRoot, source);
  const destinationPath = path.join(stagingRoot, source);
  try {
    await ensureDir(path.dirname(destinationPath));
    await fs.copyFile(sourcePath, destinationPath);
  } catch {
    // Skip if file doesn't exist
  }
}

async function buildBundle({ bundleName, entries, rootFiles, timestamp, sha, contentsRootLabel }) {
  const shaSuffix = sha ? `_${sha}` : '';
  const zipName = `Intermediary_${bundleName}_${timestamp}${shaSuffix}.zip`;
  const zipPath = path.join(outputRoot, zipName);
  let stagedFiles = [];
  let stagedBytes = 0;

  await ensureDir(outputRoot);
  await removePath(zipPath);

  const stagingRoot = path.join(outputRoot, `_staging_${bundleName}_${Date.now()}`);
  try {
    await ensureDir(stagingRoot);
    for (const entry of entries) {
      await copyEntry(entry, stagingRoot);
    }
    if (rootFiles) {
      for (const file of rootFiles) {
        await copyFile(file, stagingRoot);
      }
    }

    await writeManifest(stagingRoot, bundleName, sha, contentsRootLabel);
    stagedFiles = await listFiles(stagingRoot);
    if (stagedFiles.length === 0) {
      throw new Error(`No files staged for ${bundleName}.`);
    }
    stagedBytes = await getTotalSize(stagedFiles);
    compressArchive(stagingRoot, zipPath);
  } finally {
    await removePath(stagingRoot);
  }

  const zipStats = await fs.stat(zipPath);
  if (zipStats.size === 0) {
    await removePath(zipPath);
    throw new Error(`Zip creation failed for ${bundleName}; output was empty.`);
  }

  const uncompressedBytes = stagedBytes;
  const mb = (bytes) => (bytes / (1024 * 1024)).toFixed(2);
  const zipBytes = zipStats.size;
  console.log(`Created: ${zipPath}`);
  console.log(
    `Size: ${mb(zipBytes)} MB zip (${mb(uncompressedBytes)} MB staged, ${stagedFiles.length} files)`,
  );

  return {
    bundleName,
    zipPath,
    zipName,
    fileCount: stagedFiles.length,
    stagedBytes: uncompressedBytes,
    zipBytes,
    timestamp,
    sha,
  };
}

async function removeOldZips(prefixes) {
  try {
    const outputReal = await fs.realpath(outputRoot);
    const entries = await fs.readdir(outputRoot);
    const zipFiles = entries.filter(
      (entry) =>
        entry.endsWith('.zip') && prefixes.some((prefix) => entry.startsWith(prefix)),
    );
    await Promise.all(
      zipFiles.map(async (entry) => {
        const targetPath = path.join(outputRoot, entry);
        const targetReal = await fs.realpath(targetPath);
        if (path.dirname(targetReal) !== outputReal) {
          throw new Error(`Refusing to delete outside output: ${targetReal}`);
        }
        if (!targetReal.endsWith('.zip')) {
          throw new Error(`Refusing to delete non-zip: ${targetReal}`);
        }
        await removePath(targetPath);
      }),
    );
  } catch {
    // Ignore cleanup errors.
  }
}

async function writeLatestCopy(bundleResult) {
  const latestName = `Intermediary_${bundleResult.bundleName}_latest.zip`;
  const latestPath = path.join(outputRoot, latestName);
  await fs.copyFile(bundleResult.zipPath, latestPath);
  return { latestName, latestPath };
}

async function writeIndex(results, meta = {}) {
  const indexPath = path.join(outputRoot, 'Intermediary_Bundles_Index.md');
  const lines = [
    '# Intermediary Bundle Index',
    '',
    `Generated: ${new Date().toISOString()}`,
    `Git SHA: ${meta.sha || results[0]?.sha || 'unavailable'}`,
    '',
    'Bundles:',
  ];
  for (const result of results) {
    const mb = (bytes) => (bytes / (1024 * 1024)).toFixed(2);
    if (typeof result.zipBytes === 'number' && typeof result.stagedBytes === 'number') {
      lines.push(
        `- ${result.bundleName}: ${result.zipName} (${mb(result.zipBytes)} MB zip, ${mb(
          result.stagedBytes,
        )} MB staged, ${result.fileCount} files)`,
      );
    } else {
      lines.push(`- ${result.bundleName}: ${result.zipName}`);
    }
  }
  lines.push('');
  lines.push('Latest:');
  for (const result of results) {
    lines.push(`- Intermediary_${result.bundleName}_latest.zip`);
  }
  lines.push('');
  lines.push('Notes:');
  lines.push('- Full bundle: complete codebase (app/, src-tauri/, docs/, scripts/, root configs)');
  lines.push('- App bundle: Frontend + Tauri only (app/, src-tauri/)');
  lines.push('- Docs bundle: Documentation only (docs/)');
  await fs.writeFile(indexPath, `${lines.join('\n')}\n`, 'utf8');
}

// Root config files to include in narrower bundles
const ROOT_FILES = [
  'Cargo.toml',
  'package.json',
  'pnpm-lock.yaml',
  'tsconfig.json',
  'vite.config.ts',
  '.gitignore',
  'README.md',
  'CLAUDE.md',
  'AGENTS.md',
];

const timestamp = formatTimestamp(new Date());
const sha = getGitShortSha();

const args = process.argv.slice(2);
const wantsFull = args.includes('--full');
const wantsApp = args.includes('--app');
const wantsDocs = args.includes('--docs');
const wantsAll = !wantsFull && !wantsApp && !wantsDocs;
const includeFull = wantsFull || wantsAll;
const includeApp = wantsApp || wantsAll;
const includeDocs = wantsDocs || wantsAll;
const latestOnly = args.includes('--latest-only');

const prefixes = [];
if (includeFull) prefixes.push('Intermediary_Full_');
if (includeApp) prefixes.push('Intermediary_App_');
if (includeDocs) prefixes.push('Intermediary_Docs_');

await removeOldZips(prefixes);

const results = [];

if (includeFull) {
  const rootFiles = await listRootFiles(repoRoot);
  const result = await buildBundle({
    bundleName: 'Full',
    entries: [
      { source: 'app', filter: true },
      { source: 'src-tauri', filter: true },
      { source: 'crates', filter: true },
      { source: 'agent', filter: true },
      { source: 'docs', filter: true },
      { source: '.vscode', filter: true },
      { source: 'scripts', filter: true },
      { source: 'logs', filter: true },
    ],
    rootFiles,
    timestamp,
    sha,
    contentsRootLabel: 'Intermediary/',
  });
  const latest = await writeLatestCopy(result);
  if (latestOnly) {
    const latestStats = await fs.stat(latest.latestPath);
    results.push({
      ...result,
      zipPath: latest.latestPath,
      zipName: latest.latestName,
      zipBytes: latestStats.size,
    });
    await removePath(result.zipPath);
  } else {
    results.push(result);
  }
}

if (includeApp) {
  const result = await buildBundle({
    bundleName: 'App',
    entries: [
      { source: 'app', filter: true },
      { source: 'src-tauri', filter: true },
    ],
    rootFiles: ['package.json', 'tsconfig.json', 'vite.config.ts', 'Cargo.toml'],
    timestamp,
    sha,
    contentsRootLabel: 'Intermediary/',
  });
  const latest = await writeLatestCopy(result);
  if (latestOnly) {
    const latestStats = await fs.stat(latest.latestPath);
    results.push({
      ...result,
      zipPath: latest.latestPath,
      zipName: latest.latestName,
      zipBytes: latestStats.size,
    });
    await removePath(result.zipPath);
  } else {
    results.push(result);
  }
}

if (includeDocs) {
  const result = await buildBundle({
    bundleName: 'Docs',
    entries: [
      { source: 'docs', filter: true },
      { source: 'scripts', filter: true },
      { source: '.vscode', filter: true },
    ],
    timestamp,
    sha,
    contentsRootLabel: 'docs/',
  });
  const latest = await writeLatestCopy(result);
  if (latestOnly) {
    const latestStats = await fs.stat(latest.latestPath);
    results.push({
      ...result,
      zipPath: latest.latestPath,
      zipName: latest.latestName,
      zipBytes: latestStats.size,
    });
    await removePath(result.zipPath);
  } else {
    results.push(result);
  }
}

await writeIndex(results, { sha });

console.log('\nDone. Bundles ready for ChatGPT context loading.');
