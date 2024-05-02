console.log("Hello, world!");
console.error("This is an error message.");

const path = "./log.txt";
try {
  const contents = await runjs.readFile(path);
  console.log("Read from a file", contents);
} catch (err) {
  console.error("Unable to read file", path, err);
}

await runjs.writeFile(path, "I can write to a file.");
const contents = await runjs.readFile(path);
console.log("Read from a file", path, "contents:", contents);
console.log("Removing file", path);
// runjs.removeFile(path);
console.log("File removed");
console.log("Hello", "runjs");
interface Foo {
  bar: string;
  fizz:Number;
}
let content
content = await runjs.fetch("https://deno.land/std@0.177.0/examples/welcome.ts");
console.log("Fetched content", content);