Rails.application.routes.draw do

  devise_for :users

  resource :belongings, only: [:show] do
    get :add_gold
    patch :add_gold_update
    get :loose_gold
    patch :loose_gold_update
  end

  resource :game_logs, only: [ :show ]
  resource :heros, only: [ :show, :update ]
  resource :items, only: [ :show, :update ]
  resource :notes, only: [:edit, :update]

  resources :bugs, only: [ :index, :show, :new, :create ]

  resource :fights, only: [ :show, :update, :new, :create, :destroy ] do
    get :fight_monster
    get :old_fights, as: :old
    get ':fight_id/old_fights_selection', action: :old_fights_selection, as: :select_old
  end

  resource :adventures, except: [ :edit, :update, :create ] do
    get :reroll
    get :play
    get 'read_choice/:page_id', action: :read_choice, as: :read_choice
    get :roll_dices
    get :list
    get ':book_key/start_book', action: :start_book, as: :start_book
    get ':adventure_id/select', action: :select_adventure, as: :select
  end

  root 'adventures#welcome'
end
