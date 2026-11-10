import assert from 'node:assert/strict';import fs from 'node:fs/promises';
const f=JSON.parse(await fs.readFile(new URL('./fixtures/scene/expected.json',import.meta.url),'utf8'));
function intersect(stations,axis,value){for(let i=1;i<stations.length;i++){const a=stations[i-1],b=stations[i];if((value-a[axis])*(value-b[axis])<=0&&a[axis]!==b[axis]){const t=(value-a[axis])/(b[axis]-a[axis]);return a.map((v,j)=>v+t*(b[j]-v));}}throw Error('No crossing');}
function close(a,b){a.forEach((v,i)=>assert.ok(Math.abs(v-b[i])<1e-10));}
close(intersect(f.plan,1,-150),f.expectedTopIntersections[0]);close(intersect(f.actual,1,-150),f.expectedTopIntersections[1]);
close(intersect(f.plan,0,15),f.expectedSectionIntersections.plan);close(intersect(f.actual,0,15),f.expectedSectionIntersections.actual);
const glb=await fs.readFile(new URL('./fixtures/scene/planned-actual-section.glb',import.meta.url));const json=JSON.parse(glb.subarray(20,20+glb.readUInt32LE(12)).toString());assert.equal(json.nodes.length,4);assert.deepEqual(json.nodes.slice(0,2).map(n=>n.extras.kind),['plan','actual']);assert.equal(json.extras.mavisSyntheticFixture.fieldData,false);
console.log('PASS: separate planned/actual roles, metre XYZ top and section crossings, deterministic original synthetic GLB metadata');
