Rails.application.routes.draw do

  devise_for :users

  resource :game_logs, only: [ :show ]
  resource :heros, only: [ :show, :update ]
  resource :items, only: [ :show, :update ]

  resource :fights, only: [ :show, :update, :new, :create ] do
    get :fight_monster
  end

  resources :adventures, except: [ :edit, :update ] do
    get :reroll
    get :play
    get 'read_choice/:page_id', action: :read_choice, as: :read_choice
    get :roll_dices
    get :die
  end

  root 'adventures#index'
end
