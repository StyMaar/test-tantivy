import { BenchmarkDefinition, BenchmarkState } from './lib/benchmarkGenerator';
import { useBenchmark } from './lib/useBenchmark'

export const Benchmark = (props: {
  definition: BenchmarkDefinition,
  state: BenchmarkState,
  onExecuted?: () => any,
  active: boolean,
}) => {
  const [stats, metadata] = useBenchmark(props.definition, props.state, props.onExecuted, props.active);

  return (
    <div className="Benchmark">
      <pre className="benchmarkStats">{stats}</pre>
      {metadata && <pre className="benchmarkMeta">{metadata}</pre>}
    </div>
  )
}
