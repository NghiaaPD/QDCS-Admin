from sentence_transformers import SentenceTransformer
import torch

device = "cuda" if torch.cuda.is_available() else "cpu"

model_name = "sentence-transformers/all-MiniLM-L12-v2"
model = SentenceTransformer(model_name).to(device)

def get_embeddings(texts):
    embeddings = model.encode(texts, convert_to_tensor=True)
    return embeddings
