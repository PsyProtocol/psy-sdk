
function toFixedMax(value: number | bigint, maxDecimalPlaces: number) {
  const [whole, decimal] = value.toString().split(".");
  if (!decimal) {
    return whole;
  }
  return `${whole}.${decimal.slice(0, maxDecimalPlaces)}`;
}
function formatBalance(balance: number | bigint, currency: string) {
  if (balance < 0) return "? " + currency;
  return `${toFixedMax(BigInt(balance) / BigInt("100000000"), 3)} ${currency}`;
}


export {
  formatBalance,
  toFixedMax,
}