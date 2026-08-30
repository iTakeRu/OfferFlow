import { invoke } from "@tauri-apps/api/core";
import type {
  Company, Job, Stage, Application, Interview, Attachment, AudioMark,
  ExtractSettings, ExtractedJob, JobDetail, Resume, UpcomingInterview, ActivityLog, ReviewNote,
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
export const createJob = (p: JobInput) => invoke<Job>("create_job", p);
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
export const addAttachment = (p: {
  interviewId: number; kind: string; filename: string; filepath: string; size: number; duration?: number | null;
}) => invoke<Attachment>("add_attachment", p);
export const deleteAttachment = (id: number) => invoke<void>("delete_attachment", { id });
export const readFileBase64 = (path: string) => invoke<string>("read_file_base64", { path });
export const importAttachment = (p: {
  interviewId: number; kind: string; sourcePath: string;
}) => invoke<Attachment>("import_attachment", p);

// ---------- AudioMark ----------
export const listAudioMarks = (interviewId: number) => invoke<AudioMark[]>("list_audio_marks", { interviewId });
export const addAudioMark = (p: {
  interviewId: number; attachmentId: number; position: number; label: string; note?: string | null;
}) => invoke<AudioMark>("add_audio_mark", p);
export const updateAudioMark = (id: number, p: { position: number; label: string; note?: string | null }) =>
  invoke<AudioMark>("update_audio_mark", { id, ...p });
export const deleteAudioMark = (id: number) => invoke<void>("delete_audio_mark", { id });

// ---------- Export ----------
export const exportExcel = (path: string) => invoke<void>("export_excel", { path });

// ---------- Extract / Fetch ----------
export const getSettings = () => invoke<ExtractSettings>("get_settings");
export const saveSettings = (settings: ExtractSettings) => invoke<void>("set_settings", { settings });
export const fetchJdText = (url: string) => invoke<string>("fetch_jd_text", { url });
export const extractJdInfo = (content: string) => invoke<ExtractedJob>("extract_jd_info", { content });
export const listOllamaModels = () => invoke<string[]>("list_ollama_models");
export const fetchMokahrJobDetail = (url: string) => invoke<JobDetail>("fetch_mokahr_job_detail", { url });
// 统一链接解析入口：自动识别 MokaHR / Lever / Greenhouse 等平台
export const fetchJobDetail = (url: string) => invoke<JobDetail>("fetch_job_detail", { url });

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

// ---------- Activity log ----------
export const listActivities = (applicationId: number) =>
  invoke<ActivityLog[]>("list_activities", { applicationId });
export const addActivityNote = (applicationId: number, detail: string) =>
  invoke<ActivityLog>("add_activity_note", { applicationId, detail });
export const listReviewNotes = () => invoke<ReviewNote[]>("list_review_notes");