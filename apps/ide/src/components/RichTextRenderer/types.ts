enum RichTextElemType {
  Annotation = "annotation",
  User = "user",
  Deposit = "deposit",
  TransactionId = "transaction_id",
  L1Address = "l1_address",
  Hash = "hash",
  LineBreak = "line_break",
}
interface IRichTextAnnotation {
  type: RichTextElemType.Annotation;
  text: string;
  annotation: string[] | string;
}
interface IRichTextUser {
  type: RichTextElemType.User;
  text: string;
  userId: string;
}
interface IRichTextDeposit {
  type: RichTextElemType.Deposit;
  text: string;
  depositId: string;
}
interface IRichTextL1Address {
  type: RichTextElemType.L1Address;
  address: string;
  text: string;
}
interface IRichTextTransactionId {
  type: RichTextElemType.TransactionId;
  text: string;
  txid: string;
}
interface IRichTextHash {
  type: RichTextElemType.Hash;
  text: string;
  hash: string;
}
interface IRichTextLineBreak {
  type: RichTextElemType.LineBreak;
}

type IRichTextElem = IRichTextL1Address | IRichTextAnnotation | IRichTextUser | IRichTextDeposit | IRichTextTransactionId | IRichTextHash | IRichTextLineBreak;
type TRichTextContent = (IRichTextElem | string)[] | string | IRichTextElem;


export {
  RichTextElemType,
}
export type {
  IRichTextElem,
  IRichTextL1Address,
  TRichTextContent,
  IRichTextAnnotation,
  IRichTextUser,
  IRichTextDeposit,
  IRichTextTransactionId,
  IRichTextHash,
  IRichTextLineBreak,
}