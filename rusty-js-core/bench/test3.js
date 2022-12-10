let t = performance.now();

function p(n) {
    for (let i = 2;i * i <= n;i++) {
        if (n % i == 0) {
            return false;
        }
    }
    return true;
}

let sum = 0;
for (let k = 2;k < 100000;k++) {
    if (p(k)) {
        sum++;
    }
}

console.log(sum);
console.log(performance.now() - t);