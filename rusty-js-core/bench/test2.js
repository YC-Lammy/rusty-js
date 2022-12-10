let t = performance.now();

let i = 0;
var a = 9;
for (i=0;i<1000000;i++){
    a += 1;
};
console.log(a);

console.log(performance.now() - t);