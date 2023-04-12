/// <reference types="./mashin.d.ts" />

import { StateRecord } from "./mashin.d.ts";

export class Mashin {}

export abstract class Backend<T> {
  abstract name: string;
  abstract version: string;
  config: T;
  abstract save(encryptedState: StateRecord): Promise<void>;
  abstract load(): Promise<StateRecord | undefined>;
  abstract close(): Promise<void>;

  constructor(config: T) {
    this.config = config;
  }
}
