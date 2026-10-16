// Render-cache crop only. Source IDs and all supported attributes survive.
// Whole triangles are retained; this intentionally does not manufacture cut faces.
export function cropGeometry(THREE, source, bounds, sourceToBounds=null) {
    const position = source.attributes.position;
    if (!position) return null;
    if (Object.keys(source.morphAttributes || {}).length) throw new Error('Crop of morph-target geometry is unsupported.');
    const inside = new Uint8Array(position.count), point = new THREE.Vector3();
    for (let i=0;i<position.count;i++) {
        point.fromBufferAttribute(position,i);
        if(sourceToBounds)point.applyMatrix4(sourceToBounds);
        inside[i] = bounds.containsPoint(point) ? 1 : 0;
    }
    const mesh = !!source.userData.isMesh;
    const oldIndices = source.index ? Array.from(source.index.array) : Array.from({length:position.count},(_,i)=>i);
    const kept = [], faces = [], groups = [], sourceFaces = [];
    if (mesh) {
        if (oldIndices.length % 3) throw new Error('Triangle index count must be divisible by three.');
        for (let i=0;i<oldIndices.length;i+=3) {
            const tri=oldIndices.slice(i,i+3);
            if (!tri.every(n=>inside[n])) continue;
            const materialIndex=(source.groups.find(g=>i>=g.start && i<g.start+g.count)||{}).materialIndex || 0;
            const previous=groups.at(-1);
            if (previous && previous.materialIndex===materialIndex) previous.count+=3;
            else groups.push({start:faces.length,count:3,materialIndex});
            faces.push(...tri); sourceFaces.push(source.userData.sourceFaceIds?.[i/3] ?? i/3);
        }
        for (const vertex of new Set(faces)) kept.push(vertex);
    } else for (let i=0;i<inside.length;i++) if(inside[i]) kept.push(i);
    if (!kept.length) return null;
    const result = new THREE.BufferGeometry();
    for (const [name,attribute] of Object.entries(source.attributes)) {
        if (attribute.count!==position.count || attribute.isInterleavedBufferAttribute) throw new Error(`Crop cannot safely preserve ${name}.`);
        const array = new attribute.array.constructor(kept.length * attribute.itemSize);
        kept.forEach((old,n)=>{for(let k=0;k<attribute.itemSize;k++) array[n*attribute.itemSize+k]=attribute.array[old*attribute.itemSize+k];});
        result.setAttribute(name,new THREE.BufferAttribute(array,attribute.itemSize,attribute.normalized));
    }
    if (mesh) {
        const remap=new Map(kept.map((old,n)=>[old,n])); result.setIndex(faces.map(old=>remap.get(old)));
        groups.forEach(g=>result.addGroup(g.start,g.count,g.materialIndex));
    }
    result.name=source.name;
    result.userData={...source.userData,sourceVertexIds:kept.map(i=>source.userData.sourceVertexIds?.[i]??i),sourceFaceIds:sourceFaces,geometryModified:true};
    if(source.userData.originalColors) {
        const a=source.userData.originalColors, out=new a.array.constructor(kept.length*a.itemSize);
        kept.forEach((old,n)=>{for(let k=0;k<a.itemSize;k++)out[n*a.itemSize+k]=a.array[old*a.itemSize+k];});
        result.userData.originalColors=new THREE.BufferAttribute(out,a.itemSize,a.normalized);
    }
    return result;
}

export function gltfRenderCache(THREE,gltf) {
    gltf.scene.updateMatrixWorld(true);
    const objects=[]; gltf.scene.traverse(o=>{if(o.isMesh||o.isPoints)objects.push(o);});
    const first=objects[0]; const origin=new THREE.Vector3();
    if(first?.geometry.attributes.position?.count) origin.fromBufferAttribute(first.geometry.attributes.position,0).applyMatrix4(first.matrixWorld);
    if(Math.max(Math.abs(origin.x),Math.abs(origin.y),Math.abs(origin.z))<100000)origin.set(0,0,0);
    const geoms=objects.map(object=>{
        if(object.isSkinnedMesh || Object.keys(object.geometry.morphAttributes||{}).length)throw new Error('Skinned and morph-target geometry needs a supported pose workflow.');
        const geometry=object.geometry.clone(), source=object.geometry.attributes.position;
        const values=new Float32Array(source.count*3), p=new THREE.Vector3();
        for(let i=0;i<source.count;i++){p.fromBufferAttribute(source,i).applyMatrix4(object.matrixWorld).sub(origin);p.toArray(values,i*3);}
        // Transform normals without narrowing translated positions before rebasing.
        geometry.applyMatrix4(new THREE.Matrix4().extractRotation(object.matrixWorld));
        geometry.setAttribute('position',new THREE.BufferAttribute(values,3));
        if(source && object.geometry.attributes.normal){const n=object.geometry.attributes.normal, a=new Float32Array(n.count*3), nm=new THREE.Matrix3().getNormalMatrix(object.matrixWorld);for(let i=0;i<n.count;i++){p.fromBufferAttribute(n,i).applyMatrix3(nm).normalize().toArray(a,i*3);}geometry.setAttribute('normal',new THREE.BufferAttribute(a,3));}
        if(object.matrixWorld.determinant()<0 && object.isMesh){const idx=geometry.index?Array.from(geometry.index.array):Array.from({length:source.count},(_,i)=>i);for(let i=0;i<idx.length;i+=3)[idx[i+1],idx[i+2]]=[idx[i+2],idx[i+1]];geometry.setIndex(idx);}
        geometry.name=object.name;
        geometry.userData={...geometry.userData,isMesh:!!object.isMesh,isPointCloud:!!object.isPoints,sourceNodeId:object.userData.id || gltf.parser.associations.get(object)?.nodes,sourceExtras:structuredClone(object.userData),sourceMaterials:(Array.isArray(object.material)?object.material:[object.material]).map(m=>m.clone()),sourceLocalOrigin:origin.toArray()};
        return geometry;
    });
    return {geometries:geoms,origin:origin.toArray(),document:gltf.parser.json,exchange:gltf.parser.json.extras?.mavisExchange || null};
}
