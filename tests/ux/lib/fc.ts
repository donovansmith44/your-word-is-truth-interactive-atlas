import fc from 'fast-check';
export const RUNS_API = Number(process.env.FC_NUM_RUNS ?? 150);
export const RUNS_UI = Number(process.env.FC_NUM_RUNS ?? 20);
export async function fcAssert<T>(prop: fc.IAsyncPropertyWithHooks<T> | fc.IAsyncProperty<T>, runs: number) {
  await fc.assert(prop as fc.IAsyncProperty<T>, { numRuns: runs, verbose: 2 });
}
