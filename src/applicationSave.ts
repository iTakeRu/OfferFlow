/** 一次表单会话中，创建已成功但刷新失败时，重试沿用原来的投递 ID。 */
export function createApplicationSaver<P>(
  initialId: number | null,
  create: (payload: P) => Promise<{ id: number }>,
  update: (id: number, payload: P) => Promise<unknown>,
) {
  let savedId = initialId;
  let saving = false;
  return async (payload: P, onSaved: () => void | Promise<void>): Promise<void> => {
    if (saving) return;
    saving = true;
    try {
      if (savedId === null) savedId = (await create(payload)).id;
      else await update(savedId, payload);
      await onSaved();
    } finally {
      saving = false;
    }
  };
}
