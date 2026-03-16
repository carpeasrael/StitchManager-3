import { invoke } from "@tauri-apps/api/core";
import type { PurchaseOrder, OrderItem, Delivery, MaterialRequirement } from "../types/index";

// ── Purchase Orders ────────────────────────────────────────────────

export async function createOrder(order: {
  orderNumber?: string;
  supplierId: number;
  projectId?: number;
  orderDate?: string;
  expectedDelivery?: string;
  shippingCost?: number;
  notes?: string;
}): Promise<PurchaseOrder> {
  return invoke("create_order", { order });
}

export async function getOrders(): Promise<PurchaseOrder[]> {
  return invoke("get_orders");
}

export async function getOrder(orderId: number): Promise<PurchaseOrder> {
  return invoke("get_order", { orderId });
}

export async function updateOrder(
  orderId: number,
  update: {
    orderNumber?: string;
    status?: string;
    projectId?: number;
    clearProjectId?: boolean;
    orderDate?: string;
    expectedDelivery?: string;
    shippingCost?: number;
    notes?: string;
  }
): Promise<PurchaseOrder> {
  return invoke("update_order", { orderId, update });
}

export async function deleteOrder(orderId: number): Promise<void> {
  return invoke("delete_order", { orderId });
}

// ── Project-Order Queries ─────────────────────────────────────────

export async function getProjectOrders(projectId: number): Promise<PurchaseOrder[]> {
  return invoke("get_project_orders", { projectId });
}

export async function getProjectRequirements(projectId: number): Promise<MaterialRequirement[]> {
  return invoke("get_project_requirements", { projectId });
}

export async function suggestOrders(projectId: number): Promise<MaterialRequirement[]> {
  return invoke("suggest_orders", { projectId });
}

// ── Order Items ────────────────────────────────────────────────────

export async function addOrderItem(
  orderId: number,
  materialId: number,
  quantityOrdered: number,
  unitPrice?: number,
  notes?: string
): Promise<OrderItem> {
  return invoke("add_order_item", { orderId, materialId, quantityOrdered, unitPrice, notes });
}

export async function getOrderItems(orderId: number): Promise<OrderItem[]> {
  return invoke("get_order_items", { orderId });
}

export async function deleteOrderItem(itemId: number): Promise<void> {
  return invoke("delete_order_item", { itemId });
}

// ── Deliveries ─────────────────────────────────────────────────────

export async function recordDelivery(
  orderId: number,
  deliveryNote: string | undefined,
  notes: string | undefined,
  items: { orderItemId: number; quantityReceived: number }[]
): Promise<Delivery> {
  return invoke("record_delivery", { orderId, deliveryNote, notes, items });
}

export async function getDeliveries(orderId: number): Promise<Delivery[]> {
  return invoke("get_deliveries", { orderId });
}
