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
      <p>{props.definition.title}</p>
      <div>
        <pre>{stats}</pre>
        {metadata && <pre>{metadata}</pre>}
      </div>
    </div>
  )
}
