
function toFixedMax(value: number, maxDecimalPlaces: number) {
  const [whole, decimal] = value.toString().split(".");
  if (!decimal) {
    return whole;
  }
  return `${whole}.${decimal.slice(0, maxDecimalPlaces)}`;
}
function formatBalance(balance: number, currency: string) {
  if (balance < 0) return "? " + currency;
  return `${toFixedMax(balance / 100_000_000, 3)} ${currency}`;
}


export {
  formatBalance,
  toFixedMax,
}