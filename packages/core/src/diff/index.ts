function simpleStateDiff<S>(oldState: S, patch: Partial<S>): {newState: S, diff: Partial<S>, changed: boolean} {
  const newState = {...oldState, ...patch};
  const diff = {} as Partial<S>;
  let changed = false;
  for (const key in patch) {
    if (oldState[key] !== patch[key]) {
      diff[key] = patch[key];
      changed = true;
    }
  }
  return {newState, diff, changed};
}

export {
  simpleStateDiff,
}