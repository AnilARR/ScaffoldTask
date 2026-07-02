export interface Profile {
  id: string;
  name: string;
  level: string;
  interests: string[];
  created_at: string;
}

export type CourseKind = "language" | "academic";

export interface Course {
  id: string;
  slug: string;
  title: string;
  kind: CourseKind;
  description: string;
}

export type ItemKind = "flashcard" | "document" | "video" | "quiz";

export interface ContentItem {
  id: string;
  course_id: string;
  kind: ItemKind;
  title: string;
  front: string;
  back: string;
  concept_ids: string[];
  tags: string[];
  difficulty: number;
  source_url?: string | null;
}

export interface Concept {
  id: string;
  course_id: string;
  name: string;
  detail?: string | null;
  sequence: number;
  prerequisites: string[];
}

export interface CourseDetail {
  course: Course;
  concepts: Concept[];
  items: ContentItem[];
}

export interface Recommendation {
  item: ContentItem;
  score: number;
  comprehensible_ratio: number;
  new_concepts: number;
}

export interface ActivityDay {
  date: string;
  count: number;
}

export interface Stats {
  profile_id: string;
  concepts_tracked: number;
  concepts_known: number;
  average_freshness: number;
  reviews_total: number;
  activity: ActivityDay[];
}

export type Rating = 1 | 2 | 3 | 4;
