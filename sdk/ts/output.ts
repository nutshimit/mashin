type AtmosValue = string | number | boolean | RecordType | undefined;
export type ObjectType = {
  [key: string]: AtmosValue;
};
export type ArrayType = RecordType[];
export type RecordType = ObjectType | ArrayType;

export type Input<T> = T;
export type Inputs = Record<string, Input<AtmosValue>>;

export type Output<T> = T;
export type Outputs = Record<string, Output<AtmosValue>>;
