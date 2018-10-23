Rails.application.routes.draw do

  devise_for :users

  resource :game_logs, only: [ :show ]
  resource :heros, only: [ :show, :update ]
  resource :items, only: [ :show, :update ]
  resource :notes, only: [:edit, :update]

  resource :fights, only: [ :show, :update, :new, :create, :destroy ] do
    get :fight_monster
    get :old_fights, as: :old
    get ':fight_id/old_fights_selection', action: :old_fights_selection, as: :select_old
  end

  resource :adventures, except: [ :edit, :update ] do
    get :reroll
    get :play
    get 'read_choice/:page_id', action: :read_choice, as: :read_choice
    get :roll_dices
    get :die
    get :list
    get ':book_key/start_book', action: :start_book, as: :start_book
  end

  root 'adventures#welcome'
end
