import { invoke } from "@tauri-apps/api/core";
import type { Experience } from "./experiences";

export const listExperiences = () => invoke<Experience[]>("list_experiences");
export const saveExperience = (item: Experience) => invoke<Experience>("save_experience", { item });
export const deleteExperience = (id: string, revision: number) => invoke<void>("delete_experience", { id, revision });
export const importExperiences = (items: Experience[]) => invoke<number>("import_experiences", { items });
import type {
  Company, Job, Stage, Application, Interview, Attachment, AudioMark,
  ExtractSettings, ExtractedJob, OllamaStatus, Resume, UpcomingInterview, ActivityLog, ReviewNote,
  SnapshotDraft, JobSnapshot, ProtectSettings, MirrorReport, DataDirInfo,
} from "./types";

// ---------- Stage ----------
export const listStages = () => invoke<Stage[]>("list_stages");
export const createStage = (name: string) => invoke<Stage>("create_stage", { name });
export const updateStage = (id: number, name: string) => invoke<Stage>("update_stage", { id, name });
export const deleteStage = (id: number) => invoke<void>("delete_stage", { id });
export const reorderStages = (ids: number[]) => invoke<void>("reorder_stages", { ids });

// ---------- Company ----------
export type CompanyInput = {
  name: string;
  industry?: string | null;
  city?: string | null;
  website?: string | null;
  notes?: string | null;
};
export const listCompanies = () => invoke<Company[]>("list_companies");
export const createCompany = (p: CompanyInput) => invoke<Company>("create_company", p);
export const updateCompany = (id: number, p: CompanyInput) => invoke<Company>("update_company", { id, ...p });
export const deleteCompany = (id: number) => invoke<void>("delete_company", { id });

// ---------- Job ----------
export type JobInput = {
  companyId: number;
  title: string;
  department?: string | null;
  location?: string | null;
  category?: string | null;
  jdUrl?: string | null;
  jdText?: string | null;
  salaryMin?: number | null;
  salaryMax?: number | null;
};
export const listJobs = (companyId?: number | null) => invoke<Job[]>("list_jobs", { companyId });
export const createJob = (p: JobInput & { allowSameTitle?: boolean }) => invoke<Job>("create_job", p);
export const updateJob = (id: number, p: JobInput) => invoke<Job>("update_job", { id, ...p });
export const deleteJob = (id: number) => invoke<void>("delete_job", { id });

// ---------- Application ----------
export type ApplicationInput = {
  jobId: number;
  channel?: string | null;
  resumeVersion?: string | null;
  stageId: number;
  status?: string | null;
  appliedAt?: string | null;
  notes?: string | null;
};
export const listApplications = () => invoke<Application[]>("list_applications");
export const createApplication = (p: ApplicationInput) => invoke<Application>("create_application", p);
export const updateApplication = (id: number, p: ApplicationInput & { status: string }) =>
  invoke<Application>("update_application", { id, ...p });
export const moveApplication = (id: number, stageId: number) =>
  invoke<Application>("move_application", { id, stageId });
export const deleteApplication = (id: number) => invoke<void>("delete_application", { id });

// ---------- Interview ----------
export type InterviewInput = {
  applicationId: number;
  round: number;
  format?: string | null;
  location?: string | null;
  interviewer?: string | null;
  scheduledAt?: string | null;
  rating?: number | null;
  status?: string | null;
  feedback?: string | null;
};
export const listInterviews = (applicationId: number) => invoke<Interview[]>("list_interviews", { applicationId });
export const createInterview = (p: InterviewInput) => invoke<Interview>("create_interview", p);
export const updateInterview = (id: number, p: InterviewInput & { status: string }) =>
  invoke<Interview>("update_interview", { id, ...p });
export const deleteInterview = (id: number) => invoke<void>("delete_interview", { id });

// ---------- Attachment ----------
export const listAttachments = (interviewId: number) => invoke<Attachment[]>("list_attachments", { interviewId });
export const deleteAttachment = (id: number) => invoke<void>("delete_attachment", { id });
export const setAttachmentDuration = (id: number, duration: number) =>
  invoke<void>("set_attachment_duration", { id, duration });
export const importAttachment = (p: {
  interviewId: number; kind: string; sourcePath: string;
}) => invoke<Attachment>("import_attachment", p);

/** 用系统默认程序打开数据目录内的简历 / 附件（后端会校验路径归属）。 */
export const openManagedPath = (path: string) => invoke<void>("open_managed_path", { path });

// ---------- AudioMark ----------
export const listAudioMarks = (interviewId: number) => invoke<AudioMark[]>("list_audio_marks", { interviewId });
export const addAudioMark = (p: {
  interviewId: number; attachmentId: number; position: number; label: string; note?: string | null;
}) => invoke<AudioMark>("add_audio_mark", p);
export const deleteAudioMark = (id: number) => invoke<void>("delete_audio_mark", { id });

// ---------- Export ----------
export const exportExcel = (path: string) => invoke<void>("export_excel", { path });

// ---------- Extract / Fetch ----------
export const getSettings = () => invoke<ExtractSettings>("get_settings");
export const saveSettings = (settings: ExtractSettings) => invoke<void>("set_settings", { settings });
export const extractJdInfo = (content: string) => invoke<ExtractedJob>("extract_jd_info", { content });
export const listOllamaModels = () => invoke<string[]>("list_ollama_models");
export const checkOllama = () => invoke<OllamaStatus>("check_ollama");
// ---------- JD 快照 ----------
// 抓取只返回草稿（原始页面暂存在 pending 目录），用户确认后再 saveJobSnapshot 入库。
export const captureSnapshotFromUrl = (url: string) =>
  invoke<SnapshotDraft>("capture_snapshot_from_url", { url });
export const captureSnapshotFromChrome = (url: string) =>
  invoke<SnapshotDraft>("capture_snapshot_from_chrome", { url });
export const captureCurrentBossJob = () => invoke<SnapshotDraft>("capture_current_boss_job");
export const captureSnapshotFromFile = (path: string) =>
  invoke<SnapshotDraft>("capture_snapshot_from_file", { path });
export const captureSnapshotFromText = (text: string, sourceUrl: string | null) =>
  invoke<SnapshotDraft>("capture_snapshot_from_text", { text, sourceUrl });
export const discardSnapshotDraft = (token: string) =>
  invoke<void>("discard_snapshot_draft", { token });
export const saveJobSnapshot = (jobId: number, d: SnapshotDraft) =>
  invoke<JobSnapshot>("save_job_snapshot", {
    jobId,
    sourceUrl: d.sourceUrl,
    sourceKind: d.sourceKind,
    text: d.text,
    token: d.token,
  });
export const listJobSnapshots = (jobId: number) =>
  invoke<JobSnapshot[]>("list_job_snapshots", { jobId });
export const deleteJobSnapshot = (id: number) => invoke<void>("delete_job_snapshot", { id });

// ---------- Resume ----------
export const listResumes = () => invoke<Resume[]>("list_resumes");
export const importResume = (p: { name: string; sourcePath: string }) =>
  invoke<Resume>("import_resume", p);
export const deleteResume = (id: number) => invoke<void>("delete_resume", { id });
export const updateResume = (id: number, name: string, tags: string | null) =>
  invoke<Resume>("update_resume", { id, name, tags });

// ---------- Upcoming interviews ----------
export const listUpcomingInterviews = (days: number) =>
  invoke<UpcomingInterview[]>("list_upcoming_interviews", { days });

// ---------- Backup ----------
export const exportBackup = (targetDir: string) => invoke<string>("export_backup", { targetDir });
export const importBackup = (sourceDir: string) => invoke<string>("import_backup", { sourceDir });

// ---------- 数据保护 ----------
export const getProtectSettings = () => invoke<ProtectSettings>("get_protect_settings");
export const saveProtectSettings = (settings: ProtectSettings) => invoke<void>("set_protect_settings", { settings });
export const syncMirrorNow = () => invoke<MirrorReport>("sync_mirror_now");
export const getMirrorStatus = () => invoke<MirrorReport | null>("get_mirror_status");
export const openTrashDir = () => invoke<void>("open_trash_dir");
export const getDataDir = () => invoke<DataDirInfo>("get_data_dir");
export const openDataDir = () => invoke<void>("open_data_dir");

// ---------- Activity log ----------
export const listActivities = (applicationId: number) =>
  invoke<ActivityLog[]>("list_activities", { applicationId });
export const addActivityNote = (applicationId: number, detail: string) =>
  invoke<ActivityLog>("add_activity_note", { applicationId, detail });
export const listReviewNotes = () => invoke<ReviewNote[]>("list_review_notes");
