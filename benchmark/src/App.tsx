import { useRef, useState } from 'react'
import './App.css'
import { Benchmark } from './benchmark';
import { generateBenchmarks } from './lib/benchmarkGenerator';

function App() {
  const [step, setStep] = useState(0);
  const benchmarkState = useRef({});

  const benchmarks = generateBenchmarks({
    documentCount: 300,
    wordPerDocument: 100,
  });

  return (
    <div className="App">
      <h1>Tantivy Benchmark</h1>
      {benchmarks.map((benchmark, i) => (
        <Benchmark
          key={i}
          definition={benchmark}
          state={benchmarkState.current as any}
          onExecuted={() => setStep(i + 1)}
          active={i <= step}
        />
      ))}
    </div>
  )
}

export default App
