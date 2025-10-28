function decimalToSats(decimal: number) {
    const [whole, fractional] = decimal.toString().split(".");
    if (fractional) {
        return parseFloat(whole + fractional.padEnd(8, "0"));
    } else {
        return parseFloat(whole + "00000000");
    }
}

function satsToDecimal(sats: number) {
    const decimalValue = sats % 100000000;
    const decimalPart = decimalValue.toString().padStart(8, "0");
    const wholePart = Math.round((sats - decimalValue) / 100000000);
    return parseFloat(wholePart + "." + decimalPart);
}

export { decimalToSats, satsToDecimal };
