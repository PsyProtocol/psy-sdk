interface IProjectMetaData {
  id: string;
  name: string;
  createdAt: number;
  lastOpenedAt: number;
  schemaVersion: string;
}

export type {
  IProjectMetaData,
}