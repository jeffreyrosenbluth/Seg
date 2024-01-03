import { invoke } from "@tauri-apps/api/tauri";
import { dialog } from "@tauri-apps/api";
import GUI from "lil-gui";

interface Picture {
  width: number;
  height: number;
  data: Uint8Array;
}

const W = 1024;
const gui = new GUI();

// Open an image and save it to the global state.
// Then display it in the main window.
async function chooseImage() {
  try {
    // Query the user for the filepath.
    const file = (await dialog.open({
      multiple: false,
      directory: false,
      filters: [
        {
          name: "Images",
          extensions: ["png", "jpeg", "jpg", "tiff", "webp"],
        },
      ],
    })) as string;

    // Open and save the image to the global state.
    try {
      const picture: Picture = await invoke("get_image", {
        path: file,
      });
      // If the image exists show it in the window.
      displayImage(picture.width, picture.height, picture.data);
    } catch (error) {
      // If the image file could not be opened, display an error.
      displayError(error as Error);
    }
  } catch (error) {
    console.error(`Error: ${error}`);
  }
}

async function generate() {
  try {
    // Run the contamination algorithm on the input image.
    const picture: Picture = await invoke("gen_image", {
      cell: controls.cellSize,
    });
    // Show the contaminated image in the window.
    displayImage(picture.width, picture.height, picture.data);
  } catch (error) {
    console.error(`Error: ${error}`);
  }
}
// Save the image as a png. The image size will match the
// original input image.
async function save() {
  try {
    const file = (await dialog.save({
      defaultPath: "contaminated.png",
      filters: [
        {
          name: "PNG",
          extensions: ["png", "jpeg", "jpg"],
        },
      ],
    })) as string;
    await invoke("save_image", {
      cell: controls.cellSize,
      path: file,
    });
  } catch (error) {
    console.error(`Error: ${error}`);
  }
}

// Controls for the gui, two sliders a picker and 3 buttons.
let controls = {
  cellSize: 10,
  chooseImage: async function () {
    chooseImage();
  },
  generate: async function () {
    generate();
  },
  save: async function () {
    save();
  },
};

gui.add(controls, "cellSize", 1, 100, 1).name("Cell Size");
gui.add(controls, "chooseImage").name("Choose Image");
gui.add(controls, "generate").name("Generate");
gui.add(controls, "save").name("Save");

// Convert the raw image data to a canvas image and put it on the canvas.
function displayImage(width: number, height: number, data: Uint8Array) {
  const splash = document.getElementById("splash");
  splash!.style.display = "none";
  const errorElement = document.getElementById("error-message");
  if (errorElement instanceof HTMLElement) {
    errorElement.textContent = "";
    errorElement.style.display = "none";
  }
  const canvas = document.querySelector("canvas") as HTMLCanvasElement;
  canvas.style.display = "block";
  const aspect = width / height;
  const ctx = canvas.getContext("2d");
  canvas.height = W / aspect;
  let clamped_data = new Uint8ClampedArray(data);
  const img_data = new ImageData(clamped_data, width, height);
  ctx!.putImageData(img_data, 0, 0);
}

function displayError(error: Error) {
  const splash = document.getElementById("splash");
  splash!.style.display = "none";
  const canvas = document.querySelector("canvas") as HTMLCanvasElement;
  canvas.style.display = "none";
  const errorElement = document.getElementById("error-message");
  if (errorElement instanceof HTMLElement) {
    errorElement.textContent = error.toString();
    errorElement.style.display = "block";
  }
}

// Toggle the control panel.
document.addEventListener("keydown", (event) => {
  if (event.key === "c" || event.key === "C") {
    gui.show(gui._hidden);
  }
});
