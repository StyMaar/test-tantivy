import React, { Fragment, useRef, useState } from 'react'
import './App.css'
import { Benchmark } from './benchmark';
import { generateBenchmarks } from './lib/benchmarkGenerator';

const generateBenchmarkWithMeta = (args: Parameters<typeof generateBenchmarks>[0]) => ({
  args,
  cases: generateBenchmarks(args),
});

const benchmarksSuites = [
  generateBenchmarkWithMeta({
    documentCount: 100,
    wordPerDocument: 100,
  }),
  generateBenchmarkWithMeta({
    documentCount: 100,
    wordPerDocument: 2000,
  }),
];

const casesPerSuite = benchmarksSuites[0].cases.length;
const totalCases = casesPerSuite * benchmarksSuites.length;

function App() {
  const [step, setStep] = useState(0);
  const benchmarkStates = useRef(benchmarksSuites.map(() => ({})));

  const onBenchmarkExecuted = (step: number) => () => {
    setStep(step + 1);
    if (step + 1 >= totalCases) {
      console.log('FINISHED');
      benchmarkStates.current.forEach((state) => {
        // Object.keys(state).forEach((key) => {
        //   // @ts-ignore
        //   state[key] = undefined;
        // });
      });
    }
  }

  return (
    <div className="App">
      <h1>Tantivy Benchmark</h1>
      <div className="benchmarks">
        {benchmarksSuites[0].cases.map((benchmark, i) => (
          <div key={i} style={{gridRow: i + 2}} className="benchmarkCaseTitle">
            {benchmark.title}
          </div>
        ))}
        {benchmarksSuites.map((suite, i) => (
          <Fragment key={i}>
            <p
              className="benchmarkSuiteTitle"
              style={{ gridColumn: i + 2 }}
            >
              {suite.args.documentCount} docs, {suite.args.wordPerDocument} words
            </p>
            {suite.cases.map((benchmark, j) => (
              <div className="benchmarkSuite" style={{gridRow: j + 2, gridColumn: i + 2}} key={j}>
                <Benchmark
                  key={i}
                  definition={benchmark}
                  state={benchmarkStates.current[i] as any}
                  onExecuted={onBenchmarkExecuted((i * suite.cases.length) + j)}
                  active={(i * suite.cases.length) + j <= step}
                />
              </div>

            ))}
          </Fragment>
        ))}
      </div>
    </div>
  )
}

export default App
