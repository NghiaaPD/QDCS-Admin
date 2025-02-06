<script>
  import { invoke } from "@tauri-apps/api/tauri";

  let fileInput;
  let files = [];
  let loading = false;
  let dropZone;
  let notification = null;
  const MAX_FILES = 5;

  function showNotification(message, type = "error") {
    notification = { message, type };
    setTimeout(() => {
      notification = null;
    }, 3000);
  }

  function handleFileSelect() {
    fileInput.click();
  }

  function handleFiles(event) {
    const selectedFiles = Array.from(event.target.files).map((file) => ({
      name: file.name,
      file: file,
    }));
    processFiles(selectedFiles);
  }

  function handleDrop(event) {
    event.preventDefault();
    const droppedFiles = Array.from(event.dataTransfer.files).map((file) => ({
      name: file.name,
      file: file,
    }));
    processFiles(droppedFiles);
  }

  function handleDragOver(event) {
    event.preventDefault();
    dropZone.classList.add("bg-gray-100");
  }

  function handleDragLeave() {
    dropZone.classList.remove("bg-gray-100");
  }

  async function processFiles(fileList) {
    if (files.length + fileList.length > MAX_FILES) {
      showNotification("Chỉ được phép tải lên tối đa 5 file");
      return;
    }

    for (const file of fileList) {
      if (!file.name.endsWith(".docx")) {
        showNotification("Chỉ chấp nhận file .docx");
        continue;
      }

      const fileExists = files.some(
        (existingFile) => existingFile.name === file.name,
      );

      if (fileExists) {
        showNotification(`File ${file.name} đã tồn tại`);
        continue;
      }

      files = [...files, file];
    }
  }

  async function processAllFiles() {
    loading = true;
    try {
      for (const file of files) {
        console.log("Processing file:", file.name);

        const arrayBuffer = await file.file.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);
        const fileBytes = Array.from(uint8Array);

        console.log("File bytes length:", fileBytes.length);

        const content = await invoke("read_docx", {
          fileData: fileBytes,
        });

        console.log("Received content:", content);

        files = files.map((f) =>
          f.name === file.name ? { ...f, content } : f,
        );
      }
      showNotification("Xử lý thành công!", "success");

      setTimeout(() => {
        files = [];
        if (fileInput) {
          fileInput.value = "";
        }
      }, 1000);
    } catch (error) {
      console.error("Error details:", error);
      showNotification(`Lỗi khi xử lý: ${error}`);
    } finally {
      loading = false;
    }
  }

  function removeFile(index) {
    files = files.filter((_, i) => i !== index);
  }
</script>

<div class="area">
  <ul class="circles">
    <li></li>
    <li></li>
    <li></li>
    <li></li>
    <li></li>
    <li></li>
    <li></li>
    <li></li>
    <li></li>
    <li></li>
  </ul>
</div>

{#if notification}
  <div class="fixed top-4 right-4 z-50 animate-fade-in">
    <div
      class="{notification.type === 'success'
        ? 'bg-green-500'
        : 'bg-red-500'} text-white px-4 py-2 rounded-lg shadow-lg flex items-center"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-5 w-5 mr-2"
        viewBox="0 0 20 20"
        fill="currentColor"
      >
        {#if notification.type === "success"}
          <path
            fill-rule="evenodd"
            d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
            clip-rule="evenodd"
          />
        {:else}
          <path
            fill-rule="evenodd"
            d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
            clip-rule="evenodd"
          />
        {/if}
      </svg>
      {notification.message}
    </div>
  </div>
{/if}

<div class="min-h-screen flex items-center justify-center">
  <div class="bg-white rounded-lg shadow-xl p-10">
    <div
      bind:this={dropZone}
      role="button"
      tabindex="0"
      on:drop={handleDrop}
      on:dragover={handleDragOver}
      on:dragleave={handleDragLeave}
      class="border-2 border-dashed border-gray-500 rounded-xl p-12 flex flex-col items-center justify-center w-[600px] transition-colors relative"
    >
      {#if loading}
        <div
          class="absolute inset-0 bg-white/80 flex items-center justify-center"
        >
          <div
            class="w-8 h-8 border-4 border-gray-400 border-t-transparent rounded-full animate-spin"
          ></div>
        </div>
      {/if}

      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-16 w-16 text-gray-400 transform hover:scale-110 transition-transform"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
        />
      </svg>
      <p class="text-gray-600 text-lg font-medium text-center">
        Kéo & thả file vào đây
      </p>
      <p class="text-gray-400 text-sm mb-2">hoặc</p>
      <button
        on:click={handleFileSelect}
        disabled={loading}
        class="px-6 py-3 bg-[#343434] text-white text-sm font-medium rounded-lg hover:bg-gray-700 transform hover:scale-105 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-opacity-50 {loading
          ? 'opacity-50 cursor-not-allowed'
          : ''}"
      >
        Chọn file
      </button>
      <input
        bind:this={fileInput}
        type="file"
        accept=".docx"
        on:change={handleFiles}
        class="hidden"
        multiple
      />
    </div>

    {#if files.length > 0}
      <div class="mt-4 space-y-2">
        {#each files as file, index}
          <div
            class="flex items-center justify-between p-2 bg-gray-100 rounded"
          >
            <div class="flex items-center">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                class="h-5 w-5 text-gray-500 mr-2"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fill-rule="evenodd"
                  d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4z"
                  clip-rule="evenodd"
                />
              </svg>
              <span class="text-sm text-gray-600">{file.name}</span>
            </div>
            <button
              on:click={() => removeFile(index)}
              aria-label={`Xóa file ${file.name}`}
              class="text-red-500 hover:text-red-700 focus:outline-none"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                class="h-5 w-5"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fill-rule="evenodd"
                  d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z"
                  clip-rule="evenodd"
                />
              </svg>
            </button>
          </div>
        {/each}

        <button
          class="w-full mt-4 px-6 py-3 bg-green-600 text-white text-sm font-medium rounded-lg hover:bg-green-700 transform hover:scale-105 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-opacity-50 disabled:opacity-50 disabled:cursor-not-allowed"
          disabled={loading}
          on:click={processAllFiles}
        >
          Xử lý
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  @keyframes fade-in {
    from {
      opacity: 0;
      transform: translateY(-1rem);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .animate-fade-in {
    animation: fade-in 0.3s ease-out;
  }
</style>
