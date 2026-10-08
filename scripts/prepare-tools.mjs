import { readFileSync, existsSync, mkdirSync, copyFileSync, readdirSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const tools = join(root, 'src-tauri/resources/tools');
const cache = join(root, 'artifacts/components');
mkdirSync(tools, { recursive: true });
mkdirSync(cache, { recursive: true });
const components = JSON.parse(readFileSync(join(root, 'src-tauri/resources/components.json'), 'utf8'));
const digest = path => createHash('sha256').update(readFileSync(path)).digest('hex');
let proxyArguments = [];
try {
  const systemProxy = execFileSync('powershell.exe', ['-NoProfile', '-Command', "$p = Get-ItemProperty -LiteralPath 'HKCU:/Software/Microsoft/Windows/CurrentVersion/Internet Settings'; if ($p.ProxyEnable -eq 1 -and $p.ProxyServer -notmatch '=|;') { $p.ProxyServer }"], { encoding: 'utf8' }).trim();
  if (systemProxy) proxyArguments = ['--proxy', `http://${systemProxy}`];
} catch { /* Direct network is supported when no static system proxy is configured. */ }
function find(directory, name) {
  for (const item of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, item.name);
    if (item.isDirectory()) { const hit = find(path, name); if (hit) return hit; }
    else if (item.name === name) return path;
  }
}
const binaryManifest = {};
for (const [name, component] of Object.entries(components)) {
  const archive = join(cache, `${name}-${component.sha256.slice(0, 12)}.${name === 'yt-dlp' ? 'exe' : 'zip'}`);
  if (!existsSync(archive) || digest(archive) !== component.sha256) {
    console.log(`Downloading ${name} ${component.version} from official release…`);
    execFileSync('curl.exe', ['-sS', '-L', '--fail', '--retry', '2', '--connect-timeout', '30', ...proxyArguments, component.url, '-o', archive], { stdio: 'inherit' });
  }
  if (digest(archive) !== component.sha256) throw new Error(`${name}: SHA256 verification failed. Refusing to bundle unverified component.`);
  const binaries = name === 'ffmpeg' ? ['ffmpeg.exe', 'ffprobe.exe'] : [`${name}.exe`];
  let directory = cache;
  if (name !== 'yt-dlp') {
    directory = join(cache, `extracted-${name}-${component.sha256.slice(0, 12)}`);
    if (!existsSync(directory)) {
      // PowerShell -File handles Unicode paths without embedding them in shell code.
      execFileSync('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', join(root, 'scripts/extract.ps1'), archive, directory], { stdio: 'inherit' });
    }
  }
  for (const binary of binaries) {
    const source = name === 'yt-dlp' ? archive : find(directory, binary);
    if (!source) throw new Error(`Missing ${binary} in official archive`);
    copyFileSync(source, join(tools, binary));
    binaryManifest[binary] = { sha256: digest(join(tools, binary)), version: component.version, source: component.source };
  }
  const bundledLicense = name !== 'yt-dlp' ? find(directory, 'LICENSE.txt') || find(directory, 'LICENSE') : undefined;
  if (bundledLicense) copyFileSync(bundledLicense, join(tools, `${name}-LICENSE.txt`));
}
writeFileSync(join(tools, 'binaries.json'), JSON.stringify(binaryManifest, null, 2));
const licenses = join(root, 'src-tauri/resources/licenses');
mkdirSync(licenses, { recursive: true });
const upstreamNotices = {
  'yt-dlp-Unlicense.txt': `https://raw.githubusercontent.com/yt-dlp/yt-dlp/${components['yt-dlp'].version}/LICENSE`,
  'yt-dlp-UPSTREAM-README.md': `https://raw.githubusercontent.com/yt-dlp/yt-dlp/${components['yt-dlp'].version}/README.md`,
  'Deno-MIT.txt': `https://raw.githubusercontent.com/denoland/deno/v${components.deno.version}/LICENSE.md`,
  'GPL-3.txt': 'https://raw.githubusercontent.com/FFmpeg/FFmpeg/master/COPYING.GPLv3',
};
for (const [name, url] of Object.entries(upstreamNotices)) {
  const destination = join(licenses, name);
  if (!existsSync(destination)) execFileSync('curl.exe', ['-sS', '-L', '--fail', '--retry', '2', '--connect-timeout', '30', '--max-time', '180', ...proxyArguments, url, '-o', destination], { stdio: 'inherit' });
  if (readFileSync(destination).length < 100) throw new Error(`Missing upstream license: ${name}`);
}
console.log('Verified all bundled download components.');
