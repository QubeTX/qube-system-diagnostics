// LICENSE.md is authoritative. Commit the generated RTF for cargo-wix users;
// --check prevents the checked-in installer copy from drifting in CI.
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const source = fileURLToPath(new URL('../LICENSE.md', import.meta.url));
const destination = fileURLToPath(new URL('../wix/License.rtf', import.meta.url));
const escapeRtf = (text) => text.replace(/[\\{}]/g, '\\$&').replace(/[^\x00-\x7f]/g,
  (character) => `\\u${character.charCodeAt(0) > 32767 ? character.charCodeAt(0) - 65536 : character.charCodeAt(0)}?`);
const paragraphs = readFileSync(source, 'utf8').trim().split(/\r?\n\s*\r?\n/).map((paragraph) => {
  const heading = /^#+ /.test(paragraph);
  const text = paragraph.replace(/^#+ /, '').replace(/^> /gm, '')
    .replace(/\r?\n/g, ' ').replace(/\[([^\]]+)\]\(#[^)]+\)/g, '$1')
    .replace(/<([^>]+)>/g, '$1').replace(/\*\*\*|\*\*|`/g, '');
  return `${heading ? '\\b ' : ''}${escapeRtf(text)}${heading ? '\\b0' : ''}\\par\n\\par\n`;
});
const rtf = '{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Segoe UI;}}\n\\viewkind4\\uc1\\pard\\f0\\fs18\n' + paragraphs.join('') + '}\n';
if (process.argv.includes('--check')) {
  if (readFileSync(destination, 'utf8').replace(/\r\n/g, '\n') !== rtf) {
    throw new Error('MSI license is stale: run node scripts/render-msi-license.mjs');
  }
  console.log('MSI license matches LICENSE.md.');
} else {
  writeFileSync(destination, rtf);
  console.log('Rendered wix/License.rtf from LICENSE.md.');
}
