function debounce(function_: any, wait = 100, options: any = {}) {
    if (typeof function_ !== "function") {
        throw new TypeError(`Expected the first parameter to be a function, got \`${typeof function_}\`.`);
    }

    if (wait < 0) {
        throw new RangeError("`wait` must not be negative.");
    }

    // TODO: Deprecate the boolean parameter at some point.
    const { immediate } = typeof options === "boolean" ? { immediate: options } : options;

    let storedContext: any;
    let storedArguments: any;
    let timeoutId: any;
    let timestamp: any;
    let result: any;

    function later() {
        const last = Date.now() - timestamp;

        if (last < wait && last >= 0) {
            timeoutId = setTimeout(later, wait - last);
        } else {
            timeoutId = undefined;

            if (!immediate) {
                const callContext = storedContext;
                const callArguments = storedArguments;
                storedContext = undefined;
                storedArguments = undefined;
                result = function_.apply(callContext, callArguments);
            }
        }
    }

    const debounced = function (...arguments_: any) {
        //@ts-ignore
        if (storedContext && this !== storedContext) {
            throw new Error("Debounced method called with different contexts.");
        }

        //@ts-ignore
        storedContext = this; // eslint-disable-line unicorn/no-this-assignment
        storedArguments = arguments_;
        timestamp = Date.now();

        const callNow = immediate && !timeoutId;

        if (!timeoutId) {
            timeoutId = setTimeout(later, wait);
        }

        if (callNow) {
            const callContext = storedContext;
            const callArguments = storedArguments;
            storedContext = undefined;
            storedArguments = undefined;
            result = function_.apply(callContext, callArguments);
        }

        return result;
    };

    debounced.clear = () => {
        if (!timeoutId) {
            return;
        }

        clearTimeout(timeoutId);
        timeoutId = undefined;
    };

    debounced.flush = () => {
        if (!timeoutId) {
            return;
        }

        const callContext = storedContext;
        const callArguments = storedArguments;
        storedContext = undefined;
        storedArguments = undefined;
        result = function_.apply(callContext, callArguments);

        clearTimeout(timeoutId);
        timeoutId = undefined;
    };

    return debounced;
}

function debouncePromise<T, U extends unknown[]>(
    generator: (...args: U) => Promise<T>,
    wait?: number,
    options?: any
): (...args: U) => any {
    const fnc = (...args: U) => {
        generator(...args).catch(console.error);
    };
    return debounce(fnc, wait, options);
}
export { debounce, debouncePromise };
