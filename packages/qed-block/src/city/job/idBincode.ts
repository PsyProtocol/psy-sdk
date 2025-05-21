import { simpleBincodeDeserialize, simpleBincodeSerialize } from "../../utils/bincode";
import { ISimpleBincodeSerializable, SimpleBincodeSerializer } from "../../utils/bincode/adapter";
import { BincodeArraySerializeHelper } from "../../utils/bincode/array";
import { IQProvingJobDataID, deserializeJobId, serializeJobId } from "./id";

class CQJobIdSerializer implements ISimpleBincodeSerializable<IQProvingJobDataID> {
  serializeBincode(obj: IQProvingJobDataID): Uint8Array {
    return serializeJobId(obj);
  }
  deserializeBincode(data: Uint8Array): { result: IQProvingJobDataID; readLength: number; } {
    return {
      result: deserializeJobId(data),
      readLength: 24,
    }
  }
  bincodeSerializedSize(obj: IQProvingJobDataID): number {
    return 24;
  }
}

const QJobIdSerializer = new SimpleBincodeSerializer<IQProvingJobDataID>(new CQJobIdSerializer());
const QJobIdArraySerializer = new BincodeArraySerializeHelper<IQProvingJobDataID>(QJobIdSerializer);

function serializeJobIdArray(arr: IQProvingJobDataID[]): Uint8Array {
  return simpleBincodeSerialize(QJobIdArraySerializer, arr);
}
function deserializeJobIdArray(data: Uint8Array): IQProvingJobDataID[] {
  return simpleBincodeDeserialize(QJobIdArraySerializer, data);
}

export {
  CQJobIdSerializer,
  QJobIdSerializer,
  QJobIdArraySerializer,
  serializeJobIdArray,
  deserializeJobIdArray,
}