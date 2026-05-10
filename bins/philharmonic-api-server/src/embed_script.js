import endpoint from "mechanics:endpoint";

function assertObject(value, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
  return value;
}

function vectorFrom(value, label) {
  if (!Array.isArray(value)) {
    throw new Error(`${label} must be an array`);
  }
  return value.map((entry, index) => {
    if (typeof entry !== "number" || !Number.isFinite(entry)) {
      throw new Error(`${label}[${index}] must be a finite number`);
    }
    return entry;
  });
}

function responseVectors(response) {
  const body = assertObject(response.body, "embed response body");
  if (Array.isArray(body.vectors)) {
    return body.vectors;
  }
  if (Array.isArray(body.embeddings)) {
    return body.embeddings;
  }
  if (Array.isArray(body.data)) {
    return body.data.map((entry, index) => {
      const object = assertObject(entry, `embed response data[${index}]`);
      return object.vector ?? object.embedding;
    });
  }
  throw new Error("embed response body missing vectors");
}

export default async function main(arg) {
  const input = assertObject(arg, "arg");
  const items = input.items;
  if (!Array.isArray(items)) {
    throw new Error("arg.items must be an array");
  }
  const maxBatchSize = input.max_batch_size;
  if (!Number.isInteger(maxBatchSize) || maxBatchSize < 1) {
    throw new Error("arg.max_batch_size must be a positive integer");
  }

  const corpus = [];
  for (let start = 0; start < items.length; start += maxBatchSize) {
    const batch = items.slice(start, start + maxBatchSize);
    const texts = batch.map((item, index) => {
      const object = assertObject(item, `arg.items[${start + index}]`);
      if (typeof object.id !== "string" || object.id.length === 0) {
        throw new Error(`arg.items[${start + index}].id must be a non-empty string`);
      }
      if (typeof object.text !== "string") {
        throw new Error(`arg.items[${start + index}].text must be a string`);
      }
      return object.text;
    });

    const response = await endpoint("embed", { body: { texts } });
    const vectors = responseVectors(response);
    if (vectors.length !== batch.length) {
      throw new Error(`embed response vector count ${vectors.length} != ${batch.length}`);
    }

    for (let index = 0; index < batch.length; index += 1) {
      const source = batch[index];
      const item = {
        id: source.id,
        vector: vectorFrom(vectors[index], `embed response vector ${start + index}`),
      };
      if (Object.prototype.hasOwnProperty.call(source, "payload")) {
        item.payload = source.payload;
      }
      corpus.push(item);
    }
  }

  return { output: corpus, context: {}, done: true };
}
