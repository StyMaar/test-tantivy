import { useEffect, useRef, useState } from 'react';
import { BenchmarkDefinition, BenchmarkState } from './benchmarkGenerator';
import { sleep } from './utils';

export const useBenchmark = (definition: BenchmarkDefinition, state: BenchmarkState, onExecuted?: () => any, active = true) => {
  const [stats, setStats] = useState<string>('waiting');
  const [metadata, setMetadata] = useState<string | null>(null);

  const executorRef = useRef(definition.executor);
  executorRef.current = definition.executor;

  useEffect(() => {
    if (!active) {
      return
    }
    (async () => {
      const durations = new Array<number>();
      for (let i = 0; i < (definition.iterations || 1); i++) {
        const start = performance.now();
        const res = await executorRef.current(state, i);
        const end = performance.now();
        durations.push(end - start);
        if (res) {
          setMetadata(res);
          await sleep(0);
        }
      }
      const total = durations.reduce((a, b) => a + b);
      const average = total / durations.length;

      setStats(!definition.iterations ? `${total.toFixed(1)}ms` : `Total: ${total.toFixed(1)}ms, avg per iteration: ${average.toFixed(1)}ms`)
      onExecuted?.();
    })();
  }, [active]);

  return [stats, metadata] as [string, string | null];
}
