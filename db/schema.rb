# This file is auto-generated from the current state of the database. Instead
# of editing this file, please use the migrations feature of Active Record to
# incrementally modify your database, and then regenerate this schema definition.
#
# Note that this schema.rb definition is the authoritative source for your
# database schema. If you need to create the application database on another
# system, you should be using db:schema:load, not running all the migrations
# from scratch. The latter is a flawed and unsustainable approach (the more migrations
# you'll amass, the slower it'll run and the greater likelihood for issues).
#
# It's strongly recommended that you check this file into your version control system.

ActiveRecord::Schema.define(version: 20181027132910) do

  # These are extensions that must be enabled in order to support this database
  enable_extension "plpgsql"

  create_table "adventures", force: :cascade do |t|
    t.integer  "book_id",                           null: false
    t.integer  "current_page_id",                   null: false
    t.integer  "hp",                                null: false
    t.integer  "strength",                          null: false
    t.integer  "gold",                              null: false
    t.integer  "waterskins",         default: 2,    null: false
    t.integer  "waterskins_max",     default: 2
    t.integer  "rations",            default: 4
    t.boolean  "charisma_avaliable", default: true
    t.datetime "created_at",                        null: false
    t.datetime "updated_at",                        null: false
    t.string   "notes"
    t.integer  "hp_max",                            null: false
    t.integer  "strength_max",                      null: false
    t.integer  "current_fight_id"
    t.integer  "user_id",                           null: false
    t.string   "items",                             null: false
    t.index ["book_id"], name: "index_adventures_on_book_id", using: :btree
    t.index ["current_page_id"], name: "index_adventures_on_current_page_id", using: :btree
    t.index ["user_id"], name: "index_adventures_on_user_id", using: :btree
  end

  create_table "books", force: :cascade do |t|
    t.string   "name",          null: false
    t.integer  "first_page_id"
    t.datetime "created_at",    null: false
    t.datetime "updated_at",    null: false
    t.string   "book_key",      null: false
    t.index ["first_page_id"], name: "index_books_on_first_page_id", using: :btree
  end

  create_table "bugs", force: :cascade do |t|
    t.integer  "user_id",                    null: false
    t.integer  "page_id",                    null: false
    t.string   "info",                       null: false
    t.boolean  "fixed",      default: false, null: false
    t.datetime "date_fixed"
    t.datetime "created_at",                 null: false
    t.datetime "updated_at",                 null: false
  end

  create_table "fights", force: :cascade do |t|
    t.integer  "book_id",                       null: false
    t.string   "opponent_1_name",               null: false
    t.integer  "opponent_1_strength", limit: 2, null: false
    t.integer  "opponent_1_life",     limit: 2, null: false
    t.integer  "opponent_1_adv",      limit: 2
    t.string   "opponent_2_name"
    t.integer  "opponent_2_strength", limit: 2
    t.integer  "opponent_2_life",     limit: 2
    t.integer  "opponent_2_adv",      limit: 2
    t.string   "opponent_3_name"
    t.integer  "opponent_3_strength", limit: 2
    t.integer  "opponent_3_life",     limit: 2
    t.integer  "opponent_3_adv",      limit: 2
    t.string   "opponent_4_name"
    t.integer  "opponent_4_strength", limit: 2
    t.integer  "opponent_4_life",     limit: 2
    t.integer  "opponent_4_adv",      limit: 2
    t.string   "opponent_5_name"
    t.integer  "opponent_5_strength", limit: 2
    t.integer  "opponent_5_life",     limit: 2
    t.integer  "opponent_5_adv",      limit: 2
    t.datetime "created_at",                    null: false
    t.datetime "updated_at",                    null: false
    t.integer  "user_id",                       null: false
    t.index ["user_id"], name: "index_fights_on_user_id", using: :btree
  end

  create_table "game_logs", force: :cascade do |t|
    t.integer  "adventure_id", null: false
    t.integer  "log_type",     null: false
    t.string   "log_data"
    t.datetime "created_at",   null: false
    t.datetime "updated_at",   null: false
    t.integer  "page_id",      null: false
    t.index ["adventure_id"], name: "index_game_logs_on_adventure_id", using: :btree
  end

  create_table "pages", force: :cascade do |t|
    t.string   "text",       null: false
    t.string   "url",        null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.integer  "book_id",    null: false
    t.string   "page_hash",  null: false
    t.index ["book_id"], name: "index_pages_on_book_id", using: :btree
    t.index ["page_hash"], name: "index_pages_on_page_hash", unique: true, using: :btree
  end

  create_table "users", force: :cascade do |t|
    t.string   "email",                  default: "", null: false
    t.string   "encrypted_password",     default: "", null: false
    t.string   "reset_password_token"
    t.datetime "reset_password_sent_at"
    t.datetime "remember_created_at"
    t.datetime "created_at",                          null: false
    t.datetime "updated_at",                          null: false
    t.integer  "current_adventure_id"
    t.index ["email"], name: "index_users_on_email", unique: true, using: :btree
    t.index ["reset_password_token"], name: "index_users_on_reset_password_token", unique: true, using: :btree
  end

  add_foreign_key "adventures", "books"
  add_foreign_key "adventures", "fights", column: "current_fight_id"
  add_foreign_key "adventures", "pages", column: "current_page_id"
  add_foreign_key "adventures", "users"
  add_foreign_key "books", "pages", column: "first_page_id"
  add_foreign_key "bugs", "pages"
  add_foreign_key "bugs", "users"
  add_foreign_key "fights", "books"
  add_foreign_key "fights", "users"
  add_foreign_key "game_logs", "adventures"
  add_foreign_key "game_logs", "pages"
  add_foreign_key "pages", "books"
  add_foreign_key "users", "adventures", column: "current_adventure_id"
end
