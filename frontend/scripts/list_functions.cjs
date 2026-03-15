const { HyperFormula } = require('hyperformula');

const hf = HyperFormula.buildEmpty({
  licenseKey: 'gpl-v3',
});

const functions = hf.getRegisteredFunctionNames();
console.log("Supported Functions:");
console.log(JSON.stringify(functions, null, 2));
