// The part of node:worker_threads that the Node adapter uses, declared here so that the package
// builds without the types of Node (it has no dependencies).
declare module 'node:worker_threads' {
  export interface ResourceLimits {
    maxOldGenerationSizeMb?: number;
    maxYoungGenerationSizeMb?: number;
    codeRangeSizeMb?: number;
    stackSizeMb?: number;
  }
  export class Worker {
    constructor(script: URL | string, options?: { resourceLimits?: ResourceLimits });
    postMessage(message: unknown): void;
    terminate(): Promise<number>;
    on(event: 'message', listener: (data: unknown) => void): this;
    on(event: 'error', listener: (error: Error) => void): this;
  }
  export const parentPort: {
    postMessage(message: unknown): void;
    on(event: 'message', listener: (data: unknown) => void): void;
  } | null;
}
