<script>
  import { invoke } from "@tauri-apps/api/tauri";
  import { writeTextFile, readTextFile } from "@tauri-apps/api/fs";
  import html2pdf from "html2pdf.js";

  let fileInput;
  let files = [];
  let loading = false;
  let dropZone;
  let notification = null;
  const MAX_FILES = 5;
  let activeTab = "database";
  let similarityThreshold = 50;
  let showResults = false;
  let similarities = [];
  let duplicateAnswers = null;
  let selectedQuestionsToKeep = [];
  let originalFileName;
  let tempFilePath;
  let fileData = null;
  let exportLoading = false;

  // Thêm biến mới
  let insertingToNewDb = false;

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
      path: file.path || "",
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
        // console.log("Processing file:", file.name);

        const arrayBuffer = await file.file.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);
        const fileBytes = Array.from(uint8Array);

        // console.log("File bytes length:", fileBytes.length);

        const content = await invoke("read_docx", {
          fileData: fileBytes,
        });

        // console.log("Received content:", content);

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
      // console.error("Error details:", error);
      showNotification(`Lỗi khi xử lý: ${error}`);
    } finally {
      loading = false;
    }
  }

  function removeFile(index) {
    files = files.filter((_, i) => i !== index);
  }

  async function handleApplyThreshold() {
    try {
      const filePath = "configs.json";

      // Tính toán với trọng số -2/35 không làm tròn
      const calculatedValue = similarityThreshold * (-2 / 35);

      // Tạo đối tượng dữ liệu chỉ với giá trị đã tính toán
      const data = {
        Value: calculatedValue,
      };

      // Ghi trực tiếp vào file JSON, ghi đè file cũ nếu có
      await writeTextFile(filePath, JSON.stringify(data, null, 2));

      showNotification("Đã lưu ngưỡng trùng thành công!", "success");
    } catch (error) {
      // console.error("Lỗi khi xử lý:", error);
      showNotification("Có lỗi xảy ra khi lưu kết quả", "error");
    }
  }

  async function processCheckFiles() {
    if (files.length === 0) {
      return;
    }

    loading = true;
    selectedQuestionsToKeep = []; // Reset danh sách câu hỏi đã chọn

    try {
      const fileArrayBuffer = await files[0].file.arrayBuffer();
      fileData = Array.from(new Uint8Array(fileArrayBuffer)); // Lưu lại dữ liệu file để dùng sau

      // Lưu tên file gốc
      originalFileName = files[0].name;

      // Lưu file tạm (nhưng chúng ta sẽ gửi lại fileData khi xuất)
      tempFilePath = await invoke("get_temp_file_path");

      const result = await invoke("fill_format_check", { fileData: fileData });

      const parsed = JSON.parse(result);
      similarities = parsed.similarities;

      // Sửa lại: Chỉ giữ lại ID của các câu KHÔNG trùng
      selectedQuestionsToKeep = similarities
        .filter((item) => {
          return (
            !item.similarity_type ||
            item.similarity_type === "" ||
            item.similarity_type === "none"
          );
        })
        .map((item) => item.id);

      console.log(
        "Danh sách ID câu không trùng sẽ giữ lại:",
        selectedQuestionsToKeep,
      );

      showResults = true;

      // Cuộn xuống phần kết quả
      setTimeout(() => {
        const resultsSection = document.getElementById("results-section");
        if (resultsSection) {
          resultsSection.scrollIntoView({ behavior: "smooth" });
        }
      }, 100);
    } catch (error) {
      showNotification(`Lỗi khi kiểm tra trùng lặp: ${error}`, "error");
      console.error("Lỗi:", error);
    } finally {
      loading = false;
    }
  }

  // Tính toán kết quả đã lọc dựa trên tab đang chọn
  $: filteredSimilarities =
    activeTab === "all"
      ? similarities
      : activeTab === "duplicate"
        ? similarities.filter(
            (s) => s.is_similar && s.similarity_type === "file",
          )
        : activeTab === "db"
          ? similarities.filter(
              (s) => s.is_similar && s.similarity_type === "database",
            )
          : similarities.filter((s) => !s.is_similar);

  // Cập nhật hàm phân tích đáp án từ cấu trúc DOCX đúng với fill_format.rs
  function parseOptions(item) {
    if (!item.answers || !Array.isArray(item.answers)) return [];

    console.log("Frontend DEBUG: Raw answers from backend:", item.answers);
    console.log(
      "Frontend DEBUG: Correct answer keys:",
      item.correct_answer_keys,
    );
    console.log("Frontend DEBUG: Full item:", item);

    return item.answers
      .map((ans) => {
        console.log("Frontend DEBUG: Processing answer:", ans);

        // Phân tích đáp án
        let parts = ans.split(". ");
        let letter = parts[0];
        let text = parts.slice(1).join(". "); // Phòng trường hợp nội dung có dấu chấm

        // Xử lý trường hợp text bắt đầu bằng letter lặp lại (như "b. advsfvsf" từ "b. b. advsfvsf")
        if (text.startsWith(letter + ". ")) {
          text = text.substring(letter.length + 2); // Loại bỏ "b. " khỏi đầu text
        }

        console.log(
          "Frontend DEBUG: Split result - letter:",
          letter,
          "text:",
          text,
        );

        // Kiểm tra đáp án có hợp lệ không
        const isEmpty = !text || text.trim() === "" || text === letter;

        // Kiểm tra đáp án có phải đáp án đúng không - so sánh lowercase
        // Backend trả về correct_answer_keys dưới dạng lowercase
        const isCorrect =
          item.correct_answer_keys &&
          item.correct_answer_keys.includes(letter.toLowerCase());

        console.log(
          "Frontend DEBUG: Checking isCorrect - letter:",
          letter,
          "lowercase:",
          letter.toLowerCase(),
          "correct_answer_keys:",
          item.correct_answer_keys,
          "isCorrect:",
          isCorrect,
        );

        const result = {
          letter,
          text,
          fullText: letter + ". " + text, // Tạo lại fullText đúng format
          isCorrect,
          isEmpty, // Thêm trường mới để dễ lọc
        };

        console.log("Frontend DEBUG: Final result:", result);
        return result;
      })
      .filter((option) => !option.isEmpty); // Lọc bỏ các đáp án trống ngay tại đây
  }

  function toggleQuestionSelection(id) {
    if (selectedQuestionsToKeep.includes(id)) {
      selectedQuestionsToKeep = selectedQuestionsToKeep.filter(
        (qId) => qId !== id,
      );
    } else {
      selectedQuestionsToKeep = [...selectedQuestionsToKeep, id];
    }
  }

  async function filterAndExportDocx() {
    if (selectedQuestionsToKeep.length === 0) {
      showNotification(
        "Không có câu hỏi nào không trùng lặp, vui lòng kiểm tra lại",
      );
      return;
    }

    // THÊM ĐIỀU KIỆN NÀY - Chỉ kiểm tra fileData khi function này được gọi trực tiếp
    // Không kiểm tra khi đang trong quá trình reload sau insert
    if (!fileData) {
      // Chỉ hiển thị lỗi nếu thực sự không có dữ liệu, không phải do reload
      if (!insertingToNewDb) {
        showNotification("Không tìm thấy dữ liệu file. Vui lòng tải lại file.");
      }
      return;
    }

    exportLoading = true;
    try {
      const filteredDocxPath = await invoke("filter_docx_with_data", {
        fileData: fileData,
        duplicateIds: selectedQuestionsToKeep,
        originalFilename: originalFileName,
      });

      showNotification(`Đã lọc và xuất file DOCX thành công!`, "success");
    } catch (error) {
      showNotification(`Lỗi khi lọc file: ${error}`);
    } finally {
      exportLoading = false;
    }
  }

  // Thêm function mới
  async function insertFilteredToNewDb() {
    if (insertingToNewDb) return;

    insertingToNewDb = true;

    try {
      showNotification(
        "Đang tìm file filtered mới nhất và tạo bản sao database...",
        "info",
      );

      const result = await invoke("insert_filtered_to_new_db");

      showNotification(result, "success");
      console.log("Insert filtered to new DB result:", result);
    } catch (error) {
      console.error("Lỗi insert filtered to new DB:", error);
      showNotification(`Lỗi: ${error}`, "error");
    } finally {
      insertingToNewDb = false;
    }
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
        : notification.type === 'info'
          ? 'bg-blue-500'
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
        {:else if notification.type === "info"}
          <path
            fill-rule="evenodd"
            d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
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
  <div class="bg-white rounded-lg shadow-xl p-10 w-[1000px]">
    <div class="flex mb-6 border-b w-full">
      <button
        class="flex-1 px-6 py-3 font-medium border-b-2 transition-colors duration-200 {activeTab ===
        'database'
          ? 'text-[#343434] border-[#343434]'
          : 'text-gray-400 border-transparent hover:text-gray-600'}"
        on:click={() => (activeTab = "database")}
      >
        Tạo Database
      </button>
      <button
        class="flex-1 px-6 py-3 font-medium border-b-2 transition-colors duration-200 {activeTab ===
        'similarity'
          ? 'text-[#343434] border-[#343434]'
          : 'text-gray-400 border-transparent hover:text-gray-600'}"
        on:click={() => (activeTab = "similarity")}
      >
        Tạo tỉ lệ trùng lặp
      </button>
      <button
        class="flex-1 px-6 py-3 font-medium border-b-2 transition-colors duration-200 {activeTab ===
        'check'
          ? 'text-[#343434] border-[#343434]'
          : 'text-gray-400 border-transparent hover:text-gray-600'}"
        on:click={() => (activeTab = "check")}
      >
        Kiểm tra trùng
      </button>
    </div>

    {#if activeTab === "database"}
      <div
        bind:this={dropZone}
        role="button"
        tabindex="0"
        on:drop={handleDrop}
        on:dragover={handleDragOver}
        on:dragleave={handleDragLeave}
        class="border-2 border-dashed border-gray-500 rounded-xl p-12 flex flex-col items-center justify-center transition-colors relative"
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
          Tải lên file
        </p>
        <p class="text-gray-400 text-sm mb-2">Docx</p>
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

          <div class="flex justify-end mt-4">
            <button
              on:click={processAllFiles}
              disabled={loading || files.length === 0}
              class="px-6 py-2 bg-[#343434] text-white text-sm font-medium rounded-lg hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-opacity-50 {loading ||
              files.length === 0
                ? 'opacity-50 cursor-not-allowed'
                : ''}"
            >
              Tạo Database
            </button>
          </div>
        </div>
      {/if}
    {:else if activeTab === "similarity"}
      <div class="p-6 bg-white rounded-lg">
        <h3 class="text-xl font-semibold mb-6">Thiết lập ngưỡng trùng lặp</h3>

        <div class="mb-6">
          <p class="text-gray-700 mb-2">
            Chọn ngưỡng tương đồng mà hệ thống sẽ coi là trùng lặp:
          </p>
          <div class="flex items-center">
            <span class="text-gray-600 mr-4">0%</span>
            <input
              type="range"
              min="0"
              max="100"
              bind:value={similarityThreshold}
              class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
            />
            <span class="text-gray-600 ml-4">{similarityThreshold}%</span>
          </div>
        </div>

        <div class="bg-gray-100 p-4 rounded-lg mb-6">
          <p class="text-sm text-gray-700">
            <strong>Lưu ý:</strong> Các giá trị tỉ lệ trùng càng cao thì yêu cầu
            độ tương đồng càng cao để phát hiện trùng lặp. Đề xuất ngưỡng từ 40%
            đến 70%.
          </p>
        </div>

        <button
          on:click={handleApplyThreshold}
          class="px-6 py-2 bg-[#343434] text-white font-medium rounded-lg hover:bg-gray-700 transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-opacity-50"
        >
          Áp dụng ngưỡng
        </button>
      </div>
    {:else if activeTab === "check"}
      <div
        bind:this={dropZone}
        role="button"
        tabindex="0"
        on:drop={handleDrop}
        on:dragover={handleDragOver}
        on:dragleave={handleDragLeave}
        class="border-2 border-dashed border-gray-500 rounded-xl p-12 flex flex-col items-center justify-center transition-colors relative"
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
          Tải lên file
        </p>
        <p class="text-gray-400 text-sm mb-2">Docx</p>
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

          <div class="flex justify-end mt-4">
            <button
              on:click={processCheckFiles}
              disabled={loading || files.length === 0}
              class="px-6 py-2 bg-[#343434] text-white text-sm font-medium rounded-lg hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-opacity-50 {loading ||
              files.length === 0
                ? 'opacity-50 cursor-not-allowed'
                : ''}"
            >
              Kiểm tra trùng
            </button>
          </div>
        </div>
      {/if}

      {#if showResults && similarities && similarities.length > 0}
        <div
          id="results-section"
          class="mt-8 bg-white p-6 rounded-lg shadow-md"
        >
          <div class="flex justify-between items-center mb-6">
            <h2 class="text-2xl font-bold">Kết quả kiểm tra</h2>
            <div class="flex space-x-2">
              <!-- Nút Lọc & Xuất DOCX -->
              <button
                on:click={filterAndExportDocx}
                class="px-4 py-2 bg-green-600 text-white text-sm font-medium rounded-lg hover:bg-green-700 focus:outline-none {exportLoading
                  ? 'opacity-70 cursor-wait'
                  : ''}"
                disabled={exportLoading}
              >
                {#if exportLoading}
                  <span
                    class="inline-block w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin mr-2"
                  ></span>
                {/if}
                Lọc & Xuất DOCX
              </button>

              <!-- Nút Insert File Filtered Vào New DB - DI CHUYỂN VÀO ĐÂY -->
              <button
                class="px-4 py-2 bg-purple-600 text-white text-sm font-medium rounded-lg hover:bg-purple-700 focus:outline-none {insertingToNewDb
                  ? 'opacity-70 cursor-wait'
                  : ''}"
                on:click={insertFilteredToNewDb}
                disabled={insertingToNewDb}
              >
                {#if insertingToNewDb}
                  <span
                    class="inline-block w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin mr-2"
                  ></span>
                  Đang Insert...
                {:else}
                  📊 Insert Vào New DB
                {/if}
              </button>
            </div>
          </div>

          <!-- Danh sách câu hỏi -->
          <div class="space-y-6">
            {#each similarities as item, index}
              <div
                class="p-5 bg-white border rounded-lg shadow-sm {item.similarity_type
                  ? 'border-red-200'
                  : 'border-green-200'}"
              >
                <div class="flex items-center justify-between mb-3">
                  <h3 class="text-lg font-semibold mb-2">Câu {item.id}</h3>
                </div>

                <!-- Nội dung câu hỏi -->
                <div class="mb-4">
                  <p class="font-medium">Câu hỏi:</p>
                  <p class="text-gray-700">{item.docx_question || ""}</p>
                </div>

                <!-- Các phương án -->
                <div class="mb-4">
                  <p class="font-medium mb-2">Các phương án:</p>

                  <div class="grid gap-3">
                    {#each parseOptions(item) as option}
                      <div
                        class="flex items-center rounded-lg border p-4 transition-all duration-300 hover:shadow-md {option.isCorrect
                          ? 'bg-gray-100 border-[#343434]'
                          : 'border-gray-200 hover:border-gray-300 hover:bg-gray-50'}"
                      >
                        <div
                          class="mr-4 flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-full border-2 {option.isCorrect
                            ? 'border-[#343434] shadow-sm'
                            : 'border-gray-300'}"
                        >
                          {#if option.isCorrect}
                            <div
                              class="h-3 w-3 rounded-full bg-[#343434]"
                            ></div>
                          {/if}
                        </div>
                        <p
                          class={option.isCorrect
                            ? "text-gray-800 font-medium"
                            : "text-gray-700"}
                        >
                          {option.fullText}
                        </p>
                      </div>
                    {/each}
                  </div>
                </div>

                <!-- Giữ lại thông tin loại trùng lặp nhưng loại bỏ nhãn "Câu trùng" -->
                <div class="mt-4 pt-3 border-t border-gray-200">
                  {#if item.similarity_type === "question"}
                    <p
                      class="font-medium text-yellow-600 py-1 px-2 rounded inline-block"
                      style="background-color: rgba(234, 179, 8, 0.1);"
                    >
                      Trùng lặp trong câu hỏi
                    </p>
                  {:else if item.similarity_type === "file"}
                    <p
                      class="font-medium text-orange-600 py-1 px-2 rounded inline-block"
                      style="background-color: rgba(249, 115, 22, 0.1);"
                    >
                      Trùng với câu hỏi trong file
                    </p>
                  {:else if item.similarity_type === "database"}
                    <p
                      class="font-medium text-red-600 py-1 px-2 rounded inline-block"
                      style="background-color: rgba(239, 68, 68, 0.1);"
                    >
                      Trùng với câu hỏi trong cơ sở dữ liệu
                    </p>
                  {:else}
                    <p
                      class="font-medium text-green-600 py-1 px-2 rounded inline-block"
                      style="background-color: rgba(22, 163, 74, 0.1);"
                    >
                      Không trùng lặp
                    </p>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  /* ...existing styles... */
</style>
