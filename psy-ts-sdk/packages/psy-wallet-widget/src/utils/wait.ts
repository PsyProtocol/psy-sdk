function waitMs(duration: number) {
    return new Promise((resolve) => {
        setTimeout(resolve, duration);
    });
}

export { waitMs };
