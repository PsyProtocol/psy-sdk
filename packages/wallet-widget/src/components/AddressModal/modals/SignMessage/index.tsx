import React, { useEffect, useState } from "react";
import { useWalletState } from "packages/wallet-widget/src/hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import styles from "./SignMessage.module.scss";
import { ActionIcon, Alert, Box, Button, CopyButton, LoadingOverlay, TextInput, Textarea, Tooltip, rem } from "@mantine/core";
import { DogeInput } from "../../../DogeInput";

import {
  DogeNetworkId,
  IDogeTransactionSigner,
  decodeAddressFull,
  TWalletAbility,
  ISignatureResult,
} from "doge-sdk";
import { waitMs } from "packages/wallet-widget/src/utils/wait";
import { IconCheck, IconCopy, IconInfoCircle } from "@tabler/icons-react";
import { SignMessageLoader } from "./SignMessageLoader";
import { SignMessageSelect, getSigningAbilities } from "./SignMessageSelect";
type TSigningAbilitiy = "sign-hash-sha256" | "sign-hash-raw";
type TSigner = (
  signer: IDogeTransactionSigner,
  message: string
) => Promise<ISignatureResult>;
type TValidator = (message: string) => string | undefined;
const SigningHelpers: Record<
  TSigningAbilitiy,
  { signer: TSigner; validator: TValidator }
> = {
  "sign-hash-sha256": {
    signer: async (signer: IDogeTransactionSigner, message: string) => {
      const result = await signer.signHash(message, true);
      return result;
    },
    validator: () => undefined,
  },
  "sign-hash-raw": {
    signer: async (signer: IDogeTransactionSigner, message: string) => {
      const result = await signer.signHash(message, false);
      return result;
    },
    validator: (message) => (message.length === 64 && /^[0-9a-fA-F]{64}$/.test(message)) ? undefined : "Message must be a 32-byte hex string",
  },
};

interface ISignMessageFormProps {
  onComplete: (result: ISignatureResult) => Promise<any>;
  signer: IDogeTransactionSigner;
  signingAbilities: TSigningAbilitiy[];
  className?: string;
}
const SignMessageForm: React.FC<ISignMessageFormProps> = ({
  signer,
  onComplete,
  className,
  signingAbilities,
}) => {
  const [signMessageType, setSignMessageType] = useState<TSigningAbilitiy>(signingAbilities[0]);
  const [message, setMessage] = useState("");
  const [messageError, setMessageError] = useState<string>();
  const [signatureResult, setSignatureResult] = useState<ISignatureResult>();
  const [loadingState, setLoadingState] = useState<
    "idle" | "loading" | "success" | "error"
  >("idle");
  const [loadingError, setLoadingError] = useState<string>();

  return (
    <div
      className={
        styles.faucetFromWalletForm + (className ? " " + className : "")
      }
    >
      <Box pos="relative">
        <SignMessageLoader
          visible={loadingState === "loading"}
          zIndex={1000}
          overlayProps={{ radius: "sm", blur: 2 }}
        />

        <div className={styles.formBody}>
          {loadingError ? (
            <Alert
              variant="light"
              color="red"
              title="Signing Error"
              icon={<IconInfoCircle />}
            >
              {loadingError}
            </Alert>
          ) : null}

          <div className={styles.inputCon}>
            <SignMessageSelect abilities={signingAbilities} value={signMessageType} onChange={(t) => {
              setSignMessageType(t);
              if (messageError) {
                setMessageError(undefined);
              }
              if (loadingError) {
                setLoadingError(undefined);
              }
              if (signatureResult) {
                setSignatureResult(undefined);
              }
            }} />
          </div>
          <div className={styles.inputCon}>
            <Textarea
              label="Message"
              placeholder="Message to sign..."
              error={messageError}
              spellCheck={false}
              onChange={(e) => {
                setMessage(e.target.value);
                if (messageError) {
                  setMessageError(undefined);
                }
                if (loadingError) {
                  setLoadingError(undefined);
                }
                if (signatureResult) {
                  setSignatureResult(undefined);
                }
              }}
              value={message}
            />
          </div>
        </div>
        <div className={styles.formControls}>
          <Button
            onClick={() => {
              if (!signMessageType) {
                return;
              }
              setLoadingError(undefined);
              setSignatureResult(undefined);
              const helper = SigningHelpers[signMessageType];

              if (!message.length) {
                setMessageError("Message is required");
                return;
              } else {
                const error = helper.validator(message);
                if (error) {
                  setMessageError(error);
                  return;
                }
              }
              setLoadingState("loading");
              helper.signer(signer, message)
                .then((result) => {
                  setSignatureResult(result);
                  setLoadingState("idle");
                  return onComplete(result);
                })
                .catch((err) => {
                  setLoadingError(err + "");
                  setLoadingState("error");
                });
            }}
            disabled={!message.length}
          >
            Sign Message
          </Button>
        </div>

        {signatureResult ? <div className={styles.signatureResultCon}>
          <div className={styles.signatureResultConInner}>
            <Textarea
              label="Signature"
              value={JSON.stringify(signatureResult, undefined, 2)}
              spellCheck={false}
              className={styles.signatureResult}
              readOnly={true}
            />
            <div className={styles.copyButtonCon}>

              <CopyButton value={JSON.stringify(signatureResult, undefined, 2)} timeout={2000}>
                {({ copied, copy }) => (
                  <Tooltip label={copied ? 'Copied' : 'Copy'} withArrow position="right">
                    <ActionIcon color={copied ? 'teal' : 'gray'} variant="subtle" onClick={copy}>
                      {copied ? (
                        <IconCheck style={{ width: rem(16) }} />
                      ) : (
                        <IconCopy style={{ width: rem(16) }} />
                      )}
                    </ActionIcon>
                  </Tooltip>
                )}
              </CopyButton>
            </div>
          </div>
        </div> : null}
      </Box>
    </div>
  );
};
const SignMessageModal: TAddressModalComponent = ({ onCancel, onComplete }) => {
  const [currentWallet, abilities] = useWalletState(
    (state) => [state.currentWallet, state.abilities]
  );
  const signingAbilities = getSigningAbilities(abilities);

  if (!currentWallet || !signingAbilities.length) {
    return <div>Message signing not enabled for this wallet.</div>;
  }
  return (
    <SignMessageForm
      signer={currentWallet.signer}
      signingAbilities={signingAbilities}
      onComplete={async () => {
        //onComplete({});
      }}
    />
  );
};

export { SignMessageModal };

export type { ISignMessageFormProps };
