import { IpcClient } from '../ipcClient';
import {
  z,
  zodValidator,
  EmptyOkResponseSchema,
  ImportApiResponseSchema,
  ImportConflictListResponseSchema,
  BatchResolveConflictsResponseSchema,
  type BatchResolveConflictsResponse,
  CancelImportBatchResponseSchema,
  type CancelImportBatchResponse,
} from '../ipcSchemas';

export const importApi = {
  async importMaterials(
    filePath: string,
    sourceBatchId: string,
    mappingProfileId?: string
  ): Promise<z.infer<typeof ImportApiResponseSchema>> {
    // 使用 snake_case 参数名（后端配置 rename_all = "snake_case"）
    return IpcClient.call(
      'import_materials',
      {
        file_path: filePath,
        source_batch_id: sourceBatchId,
        mapping_profile_id: mappingProfileId,
      },
      {
        validate: zodValidator(ImportApiResponseSchema, 'import_materials'),
      }
    );
  },

  async listImportConflicts(
    status?: string,
    limit: number = 50,
    offset: number = 0,
    batchId?: string
  ): Promise<z.infer<typeof ImportConflictListResponseSchema>> {
    return IpcClient.call('list_import_conflicts', { status, limit, offset, batch_id: batchId }, {
      validate: zodValidator(ImportConflictListResponseSchema, 'list_import_conflicts'),
    });
  },

  async resolveImportConflict(
    conflictId: string,
    action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE',
    note?: string,
    operator: string = 'system'
  ): Promise<void> {
    await IpcClient.call(
      'resolve_import_conflict',
      {
        conflict_id: conflictId,
        action,
        note,
        operator,
      },
      {
        validate: zodValidator(EmptyOkResponseSchema, 'resolve_import_conflict'),
      }
    );
  },

  /**
   * 批量处理导入冲突
   * @param conflictIds 冲突ID列表
   * @param action 处理动作 (KEEP_EXISTING | OVERWRITE | MERGE)
   * @param note 处理备注（可选）
   * @param operator 操作人（可选，默认为 'system'）
   */
  async batchResolveImportConflicts(
    conflictIds: string[],
    action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE',
    note?: string,
    operator: string = 'system'
  ): Promise<BatchResolveConflictsResponse> {
    return IpcClient.call(
      'batch_resolve_import_conflicts',
      {
        conflict_ids: conflictIds,
        action,
        note,
        operator,
      },
      {
        validate: zodValidator(BatchResolveConflictsResponseSchema, 'batch_resolve_import_conflicts'),
      }
    );
  },

  /**
   * 取消导入批次
   * @param batchId 批次ID
   * @param operator 操作人（可选，默认为 'system'）
   */
  async cancelImportBatch(batchId: string, operator: string = 'system'): Promise<CancelImportBatchResponse> {
    return IpcClient.call(
      'cancel_import_batch',
      {
        batch_id: batchId,
        operator,
      },
      {
        validate: zodValidator(CancelImportBatchResponseSchema, 'cancel_import_batch'),
      }
    );
  },
};

