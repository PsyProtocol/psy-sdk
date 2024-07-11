
import {WalletWidget, WidgetDogeWalletProvider} from '@qstudio/wallet-widget';
import { DogeMemoryWalletProvider } from 'doge-sdk';
function getProvider(){

  const provider = new DogeMemoryWalletProvider();
  provider.addRandomWallet("dogeRegtest");
  provider.addRandomWallet("dogeRegtest");
  provider.addRandomWallet("dogeRegtest");
  provider.addRandomWallet("dogeRegtest");
  provider.addRandomWallet("dogeRegtest");

  return WidgetDogeWalletProvider.fromMemoryProvider(provider);
}
const HomePage: React.FC = () => {



  return (
    <WalletWidget provider={getProvider()} />
  );
};

export default HomePage;