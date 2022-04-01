import words from 'rword/words/small.json';

export const randomWords = (wordsCount: number) => new Array(wordsCount).fill(0).map(() => words[Math.floor(words.length * Math.random())]).join(' ')
