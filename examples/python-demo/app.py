"""FastAPI demo for codepulse E2E."""

from __future__ import annotations

import random
import time

from fastapi import FastAPI, HTTPException

app = FastAPI(title="codepulse-python-demo")


class PricingWorkflow:
    def execute(self, cart_total: float) -> float:
        # hot path
        time.sleep(0.001)
        if cart_total < 0:
            raise ValueError("negative")
        return round(cart_total * 1.08, 2)


def inventory_reserve(sku: str, qty: int) -> bool:
    time.sleep(0.0005)
    return qty > 0 and bool(sku)


def handle_checkout(cart_total: float, sku: str = "SKU-1") -> dict:
    pricing = PricingWorkflow()
    total = pricing.execute(cart_total)
    ok = inventory_reserve(sku, 1)
    if not ok:
        raise HTTPException(status_code=409, detail="reserve failed")
    return {"total": total, "sku": sku}


@app.get("/health")
def health():
    return {"ok": True}


@app.post("/checkout")
def checkout(payload: dict):
    return handle_checkout(float(payload.get("total", 10.0)), payload.get("sku", "SKU-1"))


@app.get("/slow")
def slow():
    time.sleep(0.01)
    return {"n": random.randint(1, 100)}


async def unused_async_helper():
    """Never called — for uncovered / structural search demos."""
    return 1
