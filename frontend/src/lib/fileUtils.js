import * as pdfjsLib from 'pdfjs-dist/build/pdf.mjs';
import pdfjsWorker from 'pdfjs-dist/build/pdf.worker.mjs?url';

if (pdfjsLib.GlobalWorkerOptions) {
  pdfjsLib.GlobalWorkerOptions.workerSrc = pdfjsWorker;
}

export const ACCEPTED_FILE_TYPES = [
  'text/plain',
  '.txt',
  '.md',
  '.log',
  '.csv',
  '.json',
  '.tsv',
  '.text',
  'application/pdf',
  '.pdf'
].join(',');

const MAX_FILE_SIZE = 2 * 1024 * 1024; // 2 MB
const MAX_TEXT_LENGTH = 20000;

const textExtensions = ['.txt', '.md', '.log', '.csv', '.json', '.tsv'];

function isPdf(file) {
  return (
    file.type === 'application/pdf' ||
    file.name?.toLowerCase().endsWith('.pdf')
  );
}

function isTextFile(file) {
  if (file.type && file.type.startsWith && file.type.startsWith('text/')) return true;
  if (file.type === 'application/json') return true;
  const lower = file.name?.toLowerCase() || '';
  return textExtensions.some((ext) => lower.endsWith(ext));
}

function sanitizeText(text) {
  if (!text) return '';
  let cleaned = text.replace(/\r\n/g, '\n').replace(/\u0000/g, '').trim();
  if (cleaned.length > MAX_TEXT_LENGTH) {
    cleaned = cleaned.slice(0, MAX_TEXT_LENGTH) + '\n...[truncated]';
  }
  return cleaned;
}

async function extractPdfText(file) {
  const data = new Uint8Array(await file.arrayBuffer());
  const loadingTask = pdfjsLib.getDocument({ data });
  const pdf = await loadingTask.promise;
  let combined = '';

  for (let pageNum = 1; pageNum <= pdf.numPages; pageNum++) {
    const page = await pdf.getPage(pageNum);
    const content = await page.getTextContent();
    const pageText = content.items
      .map((item) => (typeof item.str === 'string' ? item.str : ''))
      .join(' ');
    combined += pageText + '\n';
    if (combined.length >= MAX_TEXT_LENGTH) {
      break;
    }
  }

  await pdf.destroy();
  return sanitizeText(combined);
}

async function extractTextFile(file) {
  const text = await file.text();
  return sanitizeText(text);
}

export async function readAttachment(file) {
  if (!file) {
    throw new Error('No file provided.');
  }

  if (file.size > MAX_FILE_SIZE) {
    throw new Error(`${file.name} is larger than 2MB.`);
  }

  if (isPdf(file)) {
    return {
      type: 'pdf',
      text: await extractPdfText(file)
    };
  }

  if (isTextFile(file)) {
    return {
      type: 'text',
      text: await extractTextFile(file)
    };
  }

  throw new Error(`${file.name} is not a supported file type.`);
}

export function formatFileSize(bytes) {
  if (!Number.isFinite(bytes)) return '';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
