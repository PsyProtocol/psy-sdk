import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const source = process.env.PSY_CONFIG_PATH || path.resolve(root, '../../../psy-genesis/config.json');
const bytes = fs.readFileSync(source);
const config = JSON.parse(bytes.toString());
if (!config.networks || !config.networks[process.env.PSY_NETWORK || 'testnet']) {
    throw new Error('SDK config does not contain the selected build network');
}
fs.copyFileSync(source, path.join(root, 'src/config/config.json'));
const magics = Object.fromEntries(['localhost', 'testnet', 'mainnet'].map(network => {
    const magic = config.networks[network]?.magic;
    if (typeof magic !== 'string' || !/^0x[0-9a-f]{16}$/i.test(magic)) {
        throw new Error(`Invalid network magic for ${network}`);
    }
    return [network, magic];
}));
fs.writeFileSync(path.join(root, 'src/action/network-magics.ts'),
    `// Generated from the selected build config; do not edit.\nexport const networkMagics = ${JSON.stringify(magics, null, 2)} as const;\n`);
