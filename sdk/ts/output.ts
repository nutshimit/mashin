type MashinValue = string | number | boolean | RecordType | undefined | null;
export type ObjectType = {
  [key: string]: MashinValue;
};
export type ArrayType = RecordType[];
export type RecordType = ObjectType | ArrayType;

export type Input<T> = T;
export type Inputs = Record<string, Input<MashinValue>>;

export type Output<T> = T;
export type Outputs = Record<string, Output<MashinValue>>;
