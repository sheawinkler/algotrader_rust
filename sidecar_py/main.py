"""FastAPI ML sidecar for AlgoTraderV2.
Provides minimal /ping, /predict, and /feature endpoints.
/predict currently returns a dummy probability until real model is integrated.
"""

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import Any, Dict
import random

app = FastAPI(title="AlgoTraderV2 ML Sidecar", version="0.1.0")


class FeaturePayload(BaseModel):
    """Generic feature payload accepted by the /predict endpoint."""

    features: Dict[str, Any]


class PredictionResponse(BaseModel):
    success: bool
    confidence: float
    label: str


@app.get("/ping")
async def ping() -> Dict[str, str]:
    """Liveness probe."""
    return {"status": "ok"}


@app.post("/predict", response_model=PredictionResponse)
async def predict(payload: FeaturePayload) -> PredictionResponse:
    """Return a dummy prediction until a real model is plugged in."""

    # TODO: replace with actual model inference
    confidence = random.uniform(0, 1)
    label = "buy" if confidence > 0.5 else "sell"
    return PredictionResponse(success=True, confidence=confidence, label=label)


@app.post("/feature")
async def feature(payload: FeaturePayload) -> Dict[str, str]:
    """Placeholder for feature engineering endpoint."""
    # Echo payload for now
    return {"received": True, "num_features": len(payload.features)}
