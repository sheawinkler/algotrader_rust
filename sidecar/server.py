"""Minimal FastAPI sidecar stub.
Run with: `uvicorn sidecar.server:app --host 0.0.0.0 --port 8000`"""
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import Any, Dict

app = FastAPI()

class PredictRequest(BaseModel):
    features: Dict[str, Any]

class PredictResponse(BaseModel):
    # For now just echo a dummy score/signal map
    signals: Dict[str, float]

@app.post("/predict", response_model=PredictResponse)
async def predict(req: PredictRequest):
    # TODO: insert ML model inference here
    return PredictResponse(signals={"score": 0.0})

@app.get("/healthz")
async def healthz():
    return {"status": "ok"}
