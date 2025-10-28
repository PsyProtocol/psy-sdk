import { Felt, PsyJSON, PsyUserWalletProvider } from "@psy/psy-sdk";
import { useCallback, useEffect, useState } from "react";


const fetchBlockNumber = async (walletProvider: PsyUserWalletProvider) => {
    try {
        console.log("Fetching latest L2 block state from coordinator...");
        const latestBlockState = await walletProvider.coordinatorEdgeRpcProvider.getLatestL2BlockState();
        console.log("Latest block state:", PsyJSON.stringify(latestBlockState, null, 2));
        if (latestBlockState) {
            return Number(latestBlockState.checkpoint_id);
        }
        return 0;
    } catch (error) {
        console.error("Failed to fetch block number from coordinator:", error);
        throw error;
    }
};

export const useBlockNumber = (walletProvider: PsyUserWalletProvider, interval: number) => {
    const [blockNumber, setBlockNumber] = useState<number>(0);

    const fetchData = useCallback(async () => {
        try {
            const number = await fetchBlockNumber(walletProvider);
            console.log("Block number fetched:", number);
            setBlockNumber(number);
        } catch (error) {
            console.error("Error fetching block number:", error);
            // Set default value to allow other hooks to continue
            setBlockNumber(1);
        }
    }, [walletProvider]);

    useEffect(() => {
        fetchData();
        const intervalId = setInterval(() => {
            fetchData();
        }, interval); // refresh every interval/1000 seconds

        return () => {
            clearInterval(intervalId);
        };
    }, [interval, fetchData]);

    return blockNumber;
};

export const fetchUserBalance = async (walletProvider: PsyUserWalletProvider, checkpointId: Felt, userId: Felt, userContractId: Felt) => {
    const merkleProof = await walletProvider.realmEdgeRpcProvider.getRpcProviderByUserId(userId).getUserContractStateTreeMerkleProof(
        checkpointId,
        userId,
        userContractId,
        32,
        0
    );
    if (merkleProof) {
        if (merkleProof.value.length != 64) {
            console.warn("fetchUserBalance failed, merkleProof.value.length != 64");
            return 0;
        }
        return parseInt(merkleProof.value?.substring(48, 64), 16);
    }
    console.warn("fetchUserBalance failed");
    return 0;
};

export const useUserBalance = (walletProvider: PsyUserWalletProvider, checkpointId: Felt, userId: Felt, userContractId: Felt, interval: number) => {
    const [balance, setBalance] = useState<Felt>(0);

    const fetchData = useCallback(async () => {
        // Only query when parameters are valid (userId can be 0)
        if (checkpointId <= 0 || userId < 0) {
            console.log("Skipping balance fetch - invalid parameters:", { checkpointId, userId, userContractId });
            setBalance(0);
            return;
        }

        try {
            console.log("Fetching balance for:", { checkpointId, userId, userContractId });
            const balance = await fetchUserBalance(walletProvider, checkpointId, userId, userContractId);
            console.log("Balance fetched:", balance);
            setBalance(balance);
        } catch (error) {
            console.error("Error fetching user balance:", error);
            setBalance(0);
        }
    }, [walletProvider, checkpointId, userId, userContractId]);

    useEffect(() => {
        fetchData();
        const intervalId = setInterval(() => {
            fetchData();
        }, interval); // refresh every interval/1000 seconds

        return () => {
            clearInterval(intervalId);
        };
    }, [interval, fetchData]);

    return balance;
};

const fetchUserId = async (walletProvider: PsyUserWalletProvider, publicKeyHex: string) => {
    return await walletProvider.coordinatorEdgeRpcProvider.getUserId(publicKeyHex);
};

export const useUserId = (walletProvider: PsyUserWalletProvider, publicKeyHex: string, interval: number) => {
    const [userId, setUserId] = useState<number>(0);

    const fetchData = useCallback(async () => {
        try {
            const userId = await fetchUserId(walletProvider, publicKeyHex);
            setUserId(userId);
        } catch (error) {
            console.error("Error fetching user id:", error);
            setUserId(0);
        }
    }, [walletProvider, publicKeyHex]);

    useEffect(() => {
        fetchData();
        const intervalId = setInterval(() => {
            fetchData();
        }, interval); // refresh every interval/1000 seconds

        return () => {
            clearInterval(intervalId);
        };
    }, [interval, fetchData]);

    return userId;
};
