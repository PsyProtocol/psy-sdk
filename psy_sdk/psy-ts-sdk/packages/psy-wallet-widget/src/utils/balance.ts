function toFixedMax(value: number | bigint, maxDecimalPlaces: number) {
    const [whole, decimal] = value.toString().split(".");
    if (!decimal) {
        return whole;
    }
    return `${whole}.${decimal.slice(0, maxDecimalPlaces)}`;
}
function formatBalance(balance: number | bigint, currency: string, decimals: number = 9) {
    if (balance < 0) return "? " + currency;
    
    // 直接用字符串操作处理大数
    const balanceStr = balance.toString();
    const divisorStr = "1" + "0".repeat(decimals); // "1000000000"
    
    if (balanceStr.length <= decimals) {
        // 小于1个单位，在前面补0
        const paddedStr = balanceStr.padStart(decimals, "0");
        const decimalPart = paddedStr.slice(-decimals);
        return `0.${decimalPart.slice(0, 3)} ${currency}`;
    } else {
        // 大于等于1个单位
        const integerPart = balanceStr.slice(0, balanceStr.length - decimals);
        const decimalPart = balanceStr.slice(-decimals);
        return `${integerPart}.${decimalPart.slice(0, 3)} ${currency}`;
    }
}

export { formatBalance, toFixedMax };
