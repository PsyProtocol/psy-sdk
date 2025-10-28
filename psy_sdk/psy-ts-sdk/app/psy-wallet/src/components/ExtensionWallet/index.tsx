import React, { useEffect } from 'react';
import { useWalletState } from "@psy/psy-wallet-widget";
import { PsyUserWalletProvider } from "@psy/psy-sdk/src/wallet/provider";
import ExtensionContent from "../../routes/home/ExtensionContent";
import { ExtensionWalletContainer } from "./ExtensionWallet.styles";

interface ExtensionWalletProps {
  provider: PsyUserWalletProvider;
}

const ExtensionWalletInner: React.FC<{ provider: PsyUserWalletProvider }> = ({ provider }) => {
  const [setWalletProvider] = useWalletState((state) => [state.setWalletProvider]);

  useEffect(() => {
    setWalletProvider(provider);
  }, [provider, setWalletProvider]);

  return (
    <ExtensionWalletContainer>
      <ExtensionContent />
    </ExtensionWalletContainer>
  );
};

export const ExtensionWallet: React.FC<ExtensionWalletProps> = ({ provider }) => {
  return <ExtensionWalletInner provider={provider} />;
};

export default ExtensionWallet;