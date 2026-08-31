export type Json =
  | string
  | number
  | boolean
  | null
  | { [key: string]: Json | undefined }
  | Json[];

export interface Database {
  public: {
    Tables: {
      arena_profiles: {
        Row: {
          id: string;
          handle: string;
          display_name: string | null;
          avatar_url: string | null;
          bio: string | null;
          link_battle_wins: number;
          link_battle_losses: number;
          link_battle_rating: number;
          total_trades: number;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id: string;
          handle: string;
          display_name?: string | null;
          avatar_url?: string | null;
          bio?: string | null;
          link_battle_wins?: number;
          link_battle_losses?: number;
          link_battle_rating?: number;
          total_trades?: number;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          handle?: string;
          display_name?: string | null;
          avatar_url?: string | null;
          bio?: string | null;
          link_battle_wins?: number;
          link_battle_losses?: number;
          link_battle_rating?: number;
          total_trades?: number;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [];
      };
      arena_agents: {
        Row: {
          id: string;
          owner_id: string;
          name: string;
          slug: string;
          description: string | null;
          repo_url: string | null;
          mcp_endpoint: string | null;
          runtime: string;
          visibility: "private" | "public";
          config: Json;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          owner_id: string;
          name: string;
          slug: string;
          description?: string | null;
          repo_url?: string | null;
          mcp_endpoint?: string | null;
          runtime?: string;
          visibility?: "private" | "public";
          config?: Json;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          owner_id?: string;
          name?: string;
          slug?: string;
          description?: string | null;
          repo_url?: string | null;
          mcp_endpoint?: string | null;
          runtime?: string;
          visibility?: "private" | "public";
          config?: Json;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [
          {
            foreignKeyName: "arena_agents_owner_id_fkey";
            columns: ["owner_id"];
            isOneToOne: false;
            referencedRelation: "users";
            referencedColumns: ["id"];
          },
        ];
      };
      arena_runs: {
        Row: {
          id: string;
          agent_id: string;
          created_by: string;
          status: "queued" | "running" | "completed" | "failed" | "cancelled";
          queue: string;
          seed: string | null;
          mcp_session_url: string | null;
          spectator_frame_url: string | null;
          started_at: string | null;
          finished_at: string | null;
          frame_count: number | null;
          badge_count: number | null;
          pokedex_seen: number | null;
          pokedex_caught: number | null;
          error: string | null;
          metrics: Json;
          notes: string | null;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          agent_id: string;
          created_by: string;
          status?: "queued" | "running" | "completed" | "failed" | "cancelled";
          queue?: string;
          seed?: string | null;
          mcp_session_url?: string | null;
          spectator_frame_url?: string | null;
          started_at?: string | null;
          finished_at?: string | null;
          frame_count?: number | null;
          badge_count?: number | null;
          pokedex_seen?: number | null;
          pokedex_caught?: number | null;
          error?: string | null;
          metrics?: Json;
          notes?: string | null;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          agent_id?: string;
          created_by?: string;
          status?: "queued" | "running" | "completed" | "failed" | "cancelled";
          queue?: string;
          seed?: string | null;
          mcp_session_url?: string | null;
          spectator_frame_url?: string | null;
          started_at?: string | null;
          finished_at?: string | null;
          frame_count?: number | null;
          badge_count?: number | null;
          pokedex_seen?: number | null;
          pokedex_caught?: number | null;
          error?: string | null;
          metrics?: Json;
          notes?: string | null;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [
          {
            foreignKeyName: "arena_runs_agent_id_fkey";
            columns: ["agent_id"];
            isOneToOne: false;
            referencedRelation: "arena_agents";
            referencedColumns: ["id"];
          },
          {
            foreignKeyName: "arena_runs_created_by_fkey";
            columns: ["created_by"];
            isOneToOne: false;
            referencedRelation: "users";
            referencedColumns: ["id"];
          },
        ];
      };
      arena_run_events: {
        Row: {
          id: number;
          run_id: string;
          frame: number | null;
          label: string | null;
          payload: Json | null;
          created_at: string;
        };
        Insert: {
          id?: number;
          run_id: string;
          frame?: number | null;
          label?: string | null;
          payload?: Json | null;
          created_at?: string;
        };
        Update: {
          id?: number;
          run_id?: string;
          frame?: number | null;
          label?: string | null;
          payload?: Json | null;
          created_at?: string;
        };
        Relationships: [
          {
            foreignKeyName: "arena_run_events_run_id_fkey";
            columns: ["run_id"];
            isOneToOne: false;
            referencedRelation: "arena_runs";
            referencedColumns: ["id"];
          },
        ];
      };
      krabbyclaw_arena_ratings: {
        Row: {
          agent_id: string;
          rating: number;
          games_played: number;
          wins: number;
          losses: number;
          draws: number;
          last_match_at: string | null;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          agent_id: string;
          rating?: number;
          games_played?: number;
          wins?: number;
          losses?: number;
          draws?: number;
          last_match_at?: string | null;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          agent_id?: string;
          rating?: number;
          games_played?: number;
          wins?: number;
          losses?: number;
          draws?: number;
          last_match_at?: string | null;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [
          {
            foreignKeyName: "krabbyclaw_arena_ratings_agent_id_fkey";
            columns: ["agent_id"];
            isOneToOne: true;
            referencedRelation: "arena_agents";
            referencedColumns: ["id"];
          },
        ];
      };
      krabbyclaw_arena_matches: {
        Row: {
          id: string;
          challenger_agent_id: string;
          opponent_agent_id: string;
          created_by: string;
          queue: string;
          status: "pending" | "running" | "completed" | "cancelled";
          outcome: "challenger" | "opponent" | "draw" | "cancelled" | null;
          winner_agent_id: string | null;
          challenger_session_id: string | null;
          opponent_session_id: string | null;
          challenger_score: number | null;
          opponent_score: number | null;
          notes: string | null;
          metadata: Json;
          started_at: string;
          finished_at: string | null;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          challenger_agent_id: string;
          opponent_agent_id: string;
          created_by: string;
          queue?: string;
          status?: "pending" | "running" | "completed" | "cancelled";
          outcome?: "challenger" | "opponent" | "draw" | "cancelled" | null;
          winner_agent_id?: string | null;
          challenger_session_id?: string | null;
          opponent_session_id?: string | null;
          challenger_score?: number | null;
          opponent_score?: number | null;
          notes?: string | null;
          metadata?: Json;
          started_at?: string;
          finished_at?: string | null;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          challenger_agent_id?: string;
          opponent_agent_id?: string;
          created_by?: string;
          queue?: string;
          status?: "pending" | "running" | "completed" | "cancelled";
          outcome?: "challenger" | "opponent" | "draw" | "cancelled" | null;
          winner_agent_id?: string | null;
          challenger_session_id?: string | null;
          opponent_session_id?: string | null;
          challenger_score?: number | null;
          opponent_score?: number | null;
          notes?: string | null;
          metadata?: Json;
          started_at?: string;
          finished_at?: string | null;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [
          {
            foreignKeyName: "krabbyclaw_arena_matches_challenger_agent_id_fkey";
            columns: ["challenger_agent_id"];
            isOneToOne: false;
            referencedRelation: "arena_agents";
            referencedColumns: ["id"];
          },
          {
            foreignKeyName: "krabbyclaw_arena_matches_opponent_agent_id_fkey";
            columns: ["opponent_agent_id"];
            isOneToOne: false;
            referencedRelation: "arena_agents";
            referencedColumns: ["id"];
          },
          {
            foreignKeyName: "krabbyclaw_arena_matches_created_by_fkey";
            columns: ["created_by"];
            isOneToOne: false;
            referencedRelation: "users";
            referencedColumns: ["id"];
          },
          {
            foreignKeyName: "krabbyclaw_arena_matches_winner_agent_id_fkey";
            columns: ["winner_agent_id"];
            isOneToOne: false;
            referencedRelation: "arena_agents";
            referencedColumns: ["id"];
          },
        ];
      };
      game_saves: {
        Row: {
          id: string;
          user_id: string;
          slot: string;
          payload: Json;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          user_id: string;
          slot: string;
          payload: Json;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          user_id?: string;
          slot?: string;
          payload?: Json;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [];
      };
      play_user_settings: {
        Row: {
          user_id: string;
          player_name: string;
          player_gender: 0 | 1;
          time_of_day: "MORN" | "DAY" | "NIGHT";
          sound_enabled: boolean;
          instant_mode_enabled: boolean;
          brand_theme:
            | "krabby"
            | "kingler"
            | "heracross"
            | "gligar"
            | "scizor"
            | "sneasel"
            | "teddiursa"
            | "ursaring"
            | "totodile"
            | "croconaw"
            | "feraligatr"
            | "pinsir";
          created_at: string;
          updated_at: string;
        };
        Insert: {
          user_id: string;
          player_name?: string;
          player_gender?: 0 | 1;
          time_of_day?: "MORN" | "DAY" | "NIGHT";
          sound_enabled?: boolean;
          instant_mode_enabled?: boolean;
          brand_theme?:
            | "krabby"
            | "kingler"
            | "heracross"
            | "gligar"
            | "scizor"
            | "sneasel"
            | "teddiursa"
            | "ursaring"
            | "totodile"
            | "croconaw"
            | "feraligatr"
            | "pinsir";
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          user_id?: string;
          player_name?: string;
          player_gender?: 0 | 1;
          time_of_day?: "MORN" | "DAY" | "NIGHT";
          sound_enabled?: boolean;
          instant_mode_enabled?: boolean;
          brand_theme?:
            | "krabby"
            | "kingler"
            | "heracross"
            | "gligar"
            | "scizor"
            | "sneasel"
            | "teddiursa"
            | "ursaring"
            | "totodile"
            | "croconaw"
            | "feraligatr"
            | "pinsir";
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [
          {
            foreignKeyName: "play_user_settings_user_id_fkey";
            columns: ["user_id"];
            isOneToOne: true;
            referencedRelation: "users";
            referencedColumns: ["id"];
          },
        ];
      };

      matchmaking_queue: {
        Row: {
          id: string;
          user_id: string;
          mode: Database["public"]["Enums"]["matchmaking_mode"];
          modpack_id: string;
          rating: number;
          party_preview: Json | null;
          preferences: Json;
          created_at: string;
          expires_at: string;
        };
        Insert: {
          id?: string;
          user_id: string;
          mode: Database["public"]["Enums"]["matchmaking_mode"];
          modpack_id?: string;
          rating?: number;
          party_preview?: Json | null;
          preferences?: Json;
          created_at?: string;
          expires_at?: string;
        };
        Update: {
          id?: string;
          user_id?: string;
          mode?: Database["public"]["Enums"]["matchmaking_mode"];
          modpack_id?: string;
          rating?: number;
          party_preview?: Json | null;
          preferences?: Json;
          created_at?: string;
          expires_at?: string;
        };
        Relationships: [
          {
            foreignKeyName: "matchmaking_queue_user_id_fkey";
            columns: ["user_id"];
            isOneToOne: false;
            referencedRelation: "users";
            referencedColumns: ["id"];
          },
        ];
      };

      matches: {
        Row: {
          id: string;
          player1_id: string;
          player2_id: string;
          mode: Database["public"]["Enums"]["matchmaking_mode"];
          modpack_id: string;
          ranked: boolean;
          status: Database["public"]["Enums"]["match_status"];
          channel_name: string;
          result: Json | null;
          created_at: string;
          started_at: string | null;
          completed_at: string | null;
        };
        Insert: {
          id?: string;
          player1_id: string;
          player2_id: string;
          mode: Database["public"]["Enums"]["matchmaking_mode"];
          modpack_id?: string;
          ranked?: boolean;
          status?: Database["public"]["Enums"]["match_status"];
          channel_name: string;
          result?: Json | null;
          created_at?: string;
          started_at?: string | null;
          completed_at?: string | null;
        };
        Update: {
          id?: string;
          player1_id?: string;
          player2_id?: string;
          mode?: Database["public"]["Enums"]["matchmaking_mode"];
          modpack_id?: string;
          ranked?: boolean;
          status?: Database["public"]["Enums"]["match_status"];
          channel_name?: string;
          result?: Json | null;
          created_at?: string;
          started_at?: string | null;
          completed_at?: string | null;
        };
        Relationships: [
          {
            foreignKeyName: "matches_player1_id_fkey";
            columns: ["player1_id"];
            isOneToOne: false;
            referencedRelation: "users";
            referencedColumns: ["id"];
          },
          {
            foreignKeyName: "matches_player2_id_fkey";
            columns: ["player2_id"];
            isOneToOne: false;
            referencedRelation: "users";
            referencedColumns: ["id"];
          },
        ];
      };

      friendships: {
        Row: {
          id: string;
          user_id: string;
          friend_id: string;
          status: Database["public"]["Enums"]["friendship_status"];
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          user_id: string;
          friend_id: string;
          status?: Database["public"]["Enums"]["friendship_status"];
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          user_id?: string;
          friend_id?: string;
          status?: Database["public"]["Enums"]["friendship_status"];
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [
          {
            foreignKeyName: "friendships_user_id_fkey";
            columns: ["user_id"];
            isOneToOne: false;
            referencedRelation: "users";
            referencedColumns: ["id"];
          },
          {
            foreignKeyName: "friendships_friend_id_fkey";
            columns: ["friend_id"];
            isOneToOne: false;
            referencedRelation: "users";
            referencedColumns: ["id"];
          },
        ];
      };
    };
    Views: {
      arena_leaderboard: {
        Row: {
          agent_id: string | null;
          best_duration: string | null;
          max_badges: number | null;
          avg_frames: number | null;
          total_runs: number | null;
        };
        Relationships: [];
      };
      krabbyclaw_arena_leaderboard: {
        Row: {
          agent_id: string | null;
          agent_name: string | null;
          agent_slug: string | null;
          runtime: string | null;
          rating: number | null;
          games_played: number | null;
          wins: number | null;
          losses: number | null;
          draws: number | null;
          win_rate: number | null;
          rank: number | null;
        };
        Relationships: [];
      };
      multiplayer_leaderboard: {
        Row: {
          id: string | null;
          handle: string | null;
          display_name: string | null;
          avatar_url: string | null;
          link_battle_wins: number | null;
          link_battle_losses: number | null;
          link_battle_rating: number | null;
          win_rate: number | null;
          total_battles: number | null;
          rank: number | null;
        };
        Relationships: [];
      };
    };
    Functions: {
      report_multiplayer_match: {
        Args: {
          report_channel_name: string;
          report_user_id: string;
          report_peer_user_id: string;
          report_mode: Database["public"]["Enums"]["matchmaking_mode"];
          report_modpack_id: string;
          report_outcome: string;
          report_metadata?: Json;
        };
        Returns: Json;
      };
    };
    Enums: {
      matchmaking_mode: "battle" | "trade" | "time_capsule";
      match_status: "waiting" | "active" | "completed" | "cancelled";
      friendship_status: "pending" | "accepted" | "blocked";
    };
    CompositeTypes: Record<string, never>;
  };
}

export type Tables<
  T extends keyof Database["public"]["Tables"]
> = Database["public"]["Tables"][T]["Row"];

export type TablesInsert<
  T extends keyof Database["public"]["Tables"]
> = Database["public"]["Tables"][T]["Insert"];

export type TablesUpdate<
  T extends keyof Database["public"]["Tables"]
> = Database["public"]["Tables"][T]["Update"];
