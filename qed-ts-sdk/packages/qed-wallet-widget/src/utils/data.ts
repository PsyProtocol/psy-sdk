import { Felt } from "@qed/qed-sdk";
import { QedUserWalletProvider } from "packages/qed-sdk/src/wallet/provider";
import { useCallback, useEffect, useState } from "react";


const fetchBlockNumber = async (walletProvider: QedUserWalletProvider) => {
    const latestBlockState = await walletProvider.coordinatorEdgeRpcProvider.getLatestL2BlockState();
    if (latestBlockState) {
        return Number(latestBlockState.checkpoint_id);
    }
    return 0;
};

export const useBlockNumber = (walletProvider: QedUserWalletProvider, interval: number) => {
    const [blockNumber, setBlockNumber] = useState<number>(0);

    const fetchData = useCallback(async () => {
        try {
            const number = await fetchBlockNumber(walletProvider);
            setBlockNumber(number);
        } catch (error) {
            console.error("Error fetching block number:", error);
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
    }, [fetchData]);

    return blockNumber;
};

export const fetchUserBalance = async (walletProvider: QedUserWalletProvider, checkpointId: Felt, userId: Felt, userContractId: Felt) => {
    console.log("fetchUserBalance:", checkpointId, userId, userContractId);
    console.log("walletProvider:", walletProvider);
    const merkleProof = await walletProvider.realmEdgeRpcProvider.getRpcProviderByUserId(userId).getUserContractStateTreeMerkleProof(
        checkpointId,
        userId,
        userContractId,
        32,
        0
    );
    if (merkleProof) {
        return parseInt(merkleProof.value, 16);
    }
    console.warn("fetchUserBalance failed");
    return 0;
};

export const useUserBalance = (walletProvider: QedUserWalletProvider, checkpointId: Felt, userId: Felt, userContractId: Felt, interval: number) => {
    const [balance, setBalance] = useState<Felt>(0);

    const fetchData = useCallback(async () => {
        try {
            const balance = await fetchUserBalance(walletProvider, checkpointId, userId, userContractId);
            setBalance(balance);
        } catch (error) {
            console.error("Error fetching user balance:", error);
            setBalance(0);
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
    }, [fetchData]);

    return balance;
};

const fetchUserId = async (walletProvider: QedUserWalletProvider, publicKeyHex: string) => {
    return await walletProvider.coordinatorEdgeRpcProvider.getUserId(publicKeyHex);
};

export const useUserId = (walletProvider: QedUserWalletProvider, publicKeyHex: string, interval: number) => {
    const [userId, setUserId] = useState<number>(0);

    const fetchData = useCallback(async () => {
        try {
            const userId = await fetchUserId(walletProvider, publicKeyHex);
            setUserId(userId);
        } catch (error) {
            console.error("Error fetching user id:", error);
            setUserId(0);
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
    }, [fetchData]);

    return userId;
};