function setInObject(obj: any, path: string[], value: any) {
    let cur = obj;
    for (let i = 0, l = path.length - 1; i < l; i++) {
        if (!Object.hasOwnProperty.call(cur, path[i])) {
            cur[path[i]] = {};
        }
    }
    cur[path[path.length - 1]] = value;
}

function getInObject(obj: any, path: string[]) {
    let cur = obj;
    for (let i = 0, l = path.length; i < l; i++) {
        if (!Object.hasOwnProperty.call(cur, path[i])) {
            return undefined;
        }
        cur = cur[path[i]];
    }
    return cur;
}

export { setInObject, getInObject };
