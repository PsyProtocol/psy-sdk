
import { SeedRandom } from '@qstudio/utils';
import {WalletWidget, WidgetDogeWalletProvider} from '@qstudio/wallet-widget';
import { DogeMemoryWalletProvider, encodePrivateKeyToWIF, hexToU8Array } from 'doge-sdk';
function getProvider(){

  const provider = new DogeMemoryWalletProvider();
  const r = new SeedRandom("cw");
  encodePrivateKeyToWIF(hexToU8Array(r.randHex(32)), "dogeRegtest")
  provider.addWalletBIP178("dogeRegtest", encodePrivateKeyToWIF(hexToU8Array(r.randHex(32)), "dogeRegtest"));
  provider.addWalletBIP178("dogeRegtest", encodePrivateKeyToWIF(hexToU8Array(r.randHex(32)), "dogeRegtest"));
  provider.addWalletBIP178("dogeRegtest", encodePrivateKeyToWIF(hexToU8Array(r.randHex(32)), "dogeRegtest"));
  provider.addWalletBIP178("dogeRegtest", encodePrivateKeyToWIF(hexToU8Array(r.randHex(32)), "dogeRegtest"));
  provider.addWalletBIP178("dogeRegtest", encodePrivateKeyToWIF(hexToU8Array(r.randHex(32)), "dogeRegtest"));

  return WidgetDogeWalletProvider.fromMemoryProvider(provider);
}
const HomePage: React.FC = () => {



  return (
    <WalletWidget provider={getProvider()} />
  );
};

export default HomePage;