export interface IMerkleProxyHelper {
    add: (a: any, b: any) => any;
    mul: (a: any, b: any) => any;
    simplify: (x: any) => any;
    getHashFelt: (index: any) => any;
    setHashFelt: (index: any, value: any) => any;
    resolveFelt: (value: any) => any;
}

export interface IFlatVariablePosition {
    name: string;
    offset: number | bigint;
    array_length: number | bigint;
    nth_size: number | bigint;
    children: IFlatVariablePosition[];
}

const arrayVariableProxy = {
    get(target: any, prop: any) {
        if (prop === Symbol.iterator) {
            return function* () {
                for (let i = BigInt(0); i < target.position.array_length; i++) {
                    yield target.helper.add(
                        target.newOffsetIndex,
                        target.helper.mul(target.position.nth_size, i)
                    );
                }
            };
        }

        if (prop === "length") {
            return target.position.array_length;
        }

        const index = BigInt(prop);
        const elementOffset = target.helper.mul(target.position.nth_size, index);
        const elementBaseOffset = target.helper.add(target.newOffsetIndex, elementOffset);

        return createVariableProxy(target.helper, target.position.children[0], elementBaseOffset);
    },
};

const structVariableProxy = {
    get(target: any, prop: any) {
        const child = target.position.children.find((x: any) => x.name === prop);
        if (!child) {
            throw new Error(`Unknown property: ${prop}`);
        }

        const fieldOffset = target.helper.add(target.newOffsetIndex, BigInt(child.offset));
        return createVariableProxy(target.helper, child, fieldOffset);
    },
};

export function isPrimitiveVariable(position: IFlatVariablePosition): boolean {
    return position.children.length === 0 && (position.nth_size === 0 || position.nth_size === BigInt(0));
}

export function isArrayVariable(position: IFlatVariablePosition): boolean {
    return position.children.length === 1 && position.children[0].name === "[]";
}

export function createVariableProxy(
    helper: IMerkleProxyHelper,
    position: IFlatVariablePosition,
    baseIndex: any
): any {
    if (isPrimitiveVariable(position)) {
        return helper.getHashFelt(baseIndex);
    }

    if (position.name === "[]") {
        if (position.children.length === 0) {
            return helper.getHashFelt(baseIndex);
        } else {
            return new Proxy({ helper, position, newOffsetIndex: baseIndex }, structVariableProxy);
        }
    }

    const newOffsetIndex =
        (position.offset === 0 || position.offset === BigInt(0)) ? baseIndex : helper.add(baseIndex, position.offset);

    if (isArrayVariable(position)) {
        return new Proxy({ helper, position, newOffsetIndex }, arrayVariableProxy);
    }

    return new Proxy({ helper, position, newOffsetIndex }, structVariableProxy);
}

export function wrapMerkleProxyHelperBasicSimplifier(helper: IMerkleProxyHelper): IMerkleProxyHelper {
    const isZero = (x: any): boolean => {
        return typeof x === "number" ? x === 0 : typeof x === "bigint" ? x === BigInt(0) : typeof x === "string" ? x === "0" : false;
    };

    const isOne = (x: any): boolean => {
        return typeof x === "number" ? x === 1 : typeof x === "bigint" ? x === BigInt(1) : typeof x === "string" ? x === "1" : false;
    };

    const isNumeric = (x: any): boolean => {
        return typeof x === "number" || typeof x === "bigint" || (typeof x === "string" && x.charCodeAt(0) >= 0x30 && x.charCodeAt(0) <= 0x39);
    };

    const simplify = (x: any) => {
        if (typeof x === "bigint") return x;
        else if (isNumeric(x)) return BigInt(x);
        else return helper.simplify(x);
    };

    const resolveFelt = (value: any) => {
        if (typeof value === "bigint") return value;
        else if (isNumeric(value)) return BigInt(value);
        else if (typeof value === "string") return helper.resolveFelt(value);
        else return value;
    };

    const add = (a: any, b: any) => {
        if (isZero(a)) return simplify(b);
        else if (isZero(b)) return simplify(a);
        else if (typeof a === "bigint" && typeof b === "bigint") return a + b;
        else if (isNumeric(a) && isNumeric(b)) return BigInt(a) + BigInt(b);
        else return helper.add(resolveFelt(a), resolveFelt(b));
    };

    const mul = (a: any, b: any) => {
        if (isZero(a) || isZero(b)) return BigInt(0);
        else if (isOne(a)) return simplify(b);
        else if (isOne(b)) return simplify(a);
        else if (typeof a === "bigint" && typeof b === "bigint") return a * b;
        else if (isNumeric(a) && isNumeric(b)) return BigInt(a) * BigInt(b);
        else return helper.mul(resolveFelt(a), resolveFelt(b));
    };

    return { add, mul, simplify, getHashFelt: helper.getHashFelt, setHashFelt: helper.setHashFelt, resolveFelt };
}
