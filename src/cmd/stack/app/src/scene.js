import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(
  75,
  window.innerWidth / window.innerHeight,
  0.1,
  1000
);
const geometry = new THREE.BoxGeometry();
const material = new THREE.MeshBasicMaterial({ color: 0x00ff00 });
const material2 = new THREE.MeshBasicMaterial({ color: 0x00ff00 });
const cube2 = new THREE.Mesh(geometry, material);
let renderer;
camera.position.z = 5;

const addOrbitControl = () => {
  const controls = new OrbitControls(camera, renderer.domElement);
  controls.target.set(0, 0, 0);
  controls.update();

  controls.addEventListener("change", animate);
};

const animate = () => {
  renderer.render(scene, camera);
};

const animateCubes = (cubes) => {
  cubes.forEach((cube) => {
    cube.rotation.x += Math.random();
    cube.rotation.y += Math.random();
    cube.position.x += Math.random() * 5;
  });

  renderer.render(scene, camera);
};

const resize = () => {
  renderer.setSize(window.innerWidth, window.innerHeight);
  camera.aspect = window.innerWidth / window.innerHeight;
  camera.updateProjectionMatrix();
};

export const createScene = (el) => {
  const cube = new THREE.Mesh(geometry, material);
  const cube2 = new THREE.Mesh(geometry, material);
  const cube3 = new THREE.Mesh(geometry, material);
  const cube4 = new THREE.Mesh(geometry, material);
  console.log(cube);
  scene.add(cube);
  scene.add(cube2);
  scene.add(cube3);
  scene.add(cube4);
  let cubes = [cube, cube2, cube3, cube4];

  renderer = new THREE.WebGLRenderer({ antialias: true, canvas: el });
  resize();
  animateCubes(cubes);
  animate();
  addOrbitControl();
};

window.addEventListener("resize", resize);
