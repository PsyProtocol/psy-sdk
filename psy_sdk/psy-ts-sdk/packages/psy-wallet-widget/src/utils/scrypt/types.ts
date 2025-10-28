enum HostMessageTypes {
    Scrypt = "Scrypt",
}
enum WorkerMessageTypes {
    ScryptResult = "ScryptResult",
    ScryptError = "ScryptError",
}
interface IScryptWorkerMessage {
    id: string;
    type: WorkerMessageTypes;
    data: string;
}
interface IScryptRequestPayloadSerialized {
    password: string;
    salt: string;
    N: number;
    r: number;
    p: number;
    dkLen: number;
}
interface IScryptRequestPayload {
    password: Uint8Array;
    salt: Uint8Array;
    N: number;
    r: number;
    p: number;
    dkLen: number;
}

export { HostMessageTypes, WorkerMessageTypes };

export type { IScryptWorkerMessage, IScryptRequestPayloadSerialized, IScryptRequestPayload };
